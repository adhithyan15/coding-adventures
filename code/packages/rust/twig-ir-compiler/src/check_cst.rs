//! `check_cst` — the validation pass that runs before lowering.
//!
//! The old pipeline lifted the CST into `twig_parser::Program` before the
//! compiler saw it, and that lift (`twig_parser::ast_extract`) was also a
//! validator: it refused integer literals that overflow `i64`, `(define x
//! a b)` with two value expressions, `(typed bogus)`, a `(Member …)` set
//! that is too large, nesting deeper than [`MAX_AST_DEPTH`], and every node
//! shape the grammar cannot produce (a hand-built tree might).  The
//! compiler now reads the CST through borrowed views (see [`crate::cst`]),
//! so those checks live here instead.
//!
//! ## Contract: the same errors, in the same order
//!
//! `check_cst` is a line-for-line mirror of the extractor's control flow
//! with the construction stripped out.  Every function below corresponds
//! to an `extract_*` function in `twig-parser/src/ast_extract.rs`, visits
//! children in the same order, increments the depth counter in the same
//! places and raises a [`TwigParseError`] with the same message and
//! position.  The first error found is therefore exactly the error the old
//! `twig_parser::parse` returned, so `compile_source` reports the same
//! `parse error: …` text as before.
//!
//! ```text
//! check_program           (root must be `program`)
//! ├─ check_module_form    (typed mode must be strict | lenient | off)
//! └─ check_form           ── depth guard
//!    ├─ check_define      ── body exprs first, then the signature
//!    │  ├─ check_fn_signature ─ check_typed_param ─ check_type_annotation
//!    │  └─ check_type_annotation (value define's `: T`)
//!    ├─ check_type_alias  ─ check_type_expr
//!    ├─ check_record_def / check_union_def ─ check_record_field
//!    └─ check_expr        ── depth guard
//!       ├─ check_atom     (INTEGER must fit in i64)
//!       ├─ check_quoted
//!       └─ check_compound ── depth guard
//!          └─ if / let / let* / begin / lambda / quote / match / apply
//! ```
//!
//! Only shape problems are reported here.  Semantic errors (unbound
//! names, captures that resolve to nothing) are still the compiler's own
//! [`crate::TwigCompileError`]s, raised during lowering exactly as before.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use twig_parser::{check_membership_int_count, TwigParseError, MAX_AST_DEPTH};

use crate::cst::{first_node, first_token_named, has_token, node_children, pos, tokens_named};

type Check = Result<(), TwigParseError>;

fn err(message: impl Into<String>, (line, column): (usize, usize)) -> TwigParseError {
    TwigParseError { message: message.into(), line, column }
}

fn check_depth(depth: usize, at: (usize, usize)) -> Check {
    if depth > MAX_AST_DEPTH {
        Err(err(
            format!(
                "AST nesting exceeds MAX_AST_DEPTH ({MAX_AST_DEPTH}) — \
                 refusing to recurse further to avoid stack overflow"
            ),
            at,
        ))
    } else {
        Ok(())
    }
}

/// Validate a Twig parse tree.  `root` must be the `program` node that
/// [`twig_parser::parse_to_ast`] returns.
///
/// # Errors
///
/// The first shape problem found, as the [`TwigParseError`] the old
/// typed-AST extractor would have raised for the same tree.
pub fn check_cst(root: &GrammarASTNode) -> Check {
    if root.rule_name != "program" {
        return Err(err(format!("expected 'program' root, got {:?}", root.rule_name), pos(root)));
    }
    for child in node_children(root) {
        match child.rule_name.as_str() {
            "module_form" => check_module_form(child)?,
            "form" => check_form(child, 0)?,
            other => {
                return Err(err(
                    format!("unexpected program child: {other:?}; expected 'module_form' or 'form'"),
                    pos(child),
                ))
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// (module name (export …) (import …) (typed …))
// ---------------------------------------------------------------------------

fn check_module_form(node: &GrammarASTNode) -> Check {
    if first_token_named(node, "NAME").is_none() {
        return Err(err("(module ...) missing module name", pos(node)));
    }
    for clause in node_children(node).filter(|c| c.rule_name == "module_clause") {
        for inner in node_children(clause).filter(|i| i.rule_name == "typed_clause") {
            if let Some(mode) = first_token_named(inner, "NAME") {
                if !matches!(mode.value.as_str(), "strict" | "lenient" | "off") {
                    return Err(err(
                        format!("(typed ...) expects strict, lenient, or off; got {:?}", mode.value),
                        (mode.line, mode.column),
                    ));
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// form = define | type_alias | record_def | union_def | expr
// ---------------------------------------------------------------------------

fn check_form(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    let inner = first_node(node).ok_or_else(|| err("empty form node", at))?;
    match inner.rule_name.as_str() {
        "define" => check_define(inner, depth + 1),
        "type_alias" => check_type_alias(inner, depth + 1),
        "record_def" => check_record_def(inner, depth + 1),
        "union_def" => check_union_def(inner, depth + 1),
        "expr" => check_expr(inner, depth + 1),
        other => Err(err(format!("unexpected form child: {other:?}"), at)),
    }
}

// ---------------------------------------------------------------------------
// Type expressions and annotations
// ---------------------------------------------------------------------------

/// Mirror of `extract_type_expr`: every `INTEGER` must fit in `i64`, and a
/// bare annotation must hold a `NAME` or `INTEGER`.
fn check_type_expr(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    let int_error = |t: &lexer::token::Token| {
        err(
            format!("integer {:?} in type expression does not fit in i64", t.value),
            (t.line, t.column),
        )
    };

    if !has_token(node, "LPAREN") {
        for child in &node.children {
            if let ASTNodeOrToken::Token(t) = child {
                match t.effective_type_name() {
                    "NAME" => return Ok(()),
                    "INTEGER" => {
                        return t.value.parse::<i64>().map(|_| ()).map_err(|_| int_error(t))
                    }
                    _ => {}
                }
            }
        }
        return Err(err("type_annotation: expected NAME or INTEGER", at));
    }

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "INTEGER" => {
                t.value.parse::<i64>().map_err(|_| int_error(t))?;
            }
            ASTNodeOrToken::Node(sub) if sub.rule_name == "type_annotation" => {
                check_type_expr(sub, depth + 1)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Mirror of `extract_type_annotation`: the type expression must be valid
/// (its errors gain a `type annotation error: ` prefix), and a
/// `(Member int (v…))` set may hold at most
/// [`twig_parser::MAX_MEMBERSHIP_INT_VALUES`] values.
fn check_type_annotation(node: &GrammarASTNode) -> Check {
    check_type_expr(node, 0).map_err(|e| TwigParseError {
        message: format!("type annotation error: {}", e.message),
        ..e
    })?;
    if let crate::cst::Annotation::MembershipInt(values) = crate::cst::classify_annotation(node) {
        let (line, column) = pos(node);
        check_membership_int_count(values.len(), line, column)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// (type …) / (record …) / (union …)
// ---------------------------------------------------------------------------

/// The first `type_annotation` node child.
fn annotation_child(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node_children(node).find(|c| c.rule_name == "type_annotation")
}

fn check_type_alias(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    if first_token_named(node, "NAME").is_none() {
        return Err(err("(type ...) missing alias name", at));
    }
    let ann = annotation_child(node).ok_or_else(|| err("(type ...) missing type expression", at))?;
    check_type_expr(ann, depth + 1)
}

fn check_record_def(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    if first_token_named(node, "NAME").is_none() {
        return Err(err("(record ...) missing type name", at));
    }
    for field in node_children(node).filter(|c| c.rule_name == "record_field") {
        check_record_field(field)?;
    }
    Ok(())
}

fn check_record_field(node: &GrammarASTNode) -> Check {
    let at = pos(node);
    let name = first_token_named(node, "NAME").ok_or_else(|| err("record field missing field name", at))?;
    let ann = annotation_child(node)
        .ok_or_else(|| err(format!("record field {:?} missing type annotation", name.value), at))?;
    check_type_annotation(ann)
}

fn check_union_def(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    if first_token_named(node, "NAME").is_none() {
        return Err(err("(union ...) missing type name", at));
    }
    for variant in node_children(node).filter(|c| c.rule_name == "union_variant") {
        if first_token_named(variant, "NAME").is_none() {
            return Err(err("union variant missing variant name", pos(variant)));
        }
        for field in node_children(variant).filter(|c| c.rule_name == "record_field") {
            check_record_field(field)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// define = LPAREN "define" name_or_signature expr { expr } RPAREN
// ---------------------------------------------------------------------------

fn check_define(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    let mut children = node_children(node);
    let sig = children.next().ok_or_else(|| err("(define ...) missing signature", at))?;
    if sig.rule_name != "name_or_signature" {
        return Err(err(format!("expected 'name_or_signature', got {:?}", sig.rule_name), at));
    }

    // The extractor lowered every body expression before looking at the
    // signature, so body errors win over signature errors.
    let mut body_count = 0usize;
    for body in children {
        check_expr(body, depth + 1)?;
        body_count += 1;
    }
    if body_count == 0 {
        return Err(err("(define ...) must have a body expression", at));
    }

    if !has_token(sig, "LPAREN") {
        // Value define: (define name expr) or (define name : T expr).
        if body_count != 1 {
            return Err(err(
                "(define name expr) takes exactly one body expression — \
                 use (define (name args...) body+) for multi-expression bodies",
                at,
            ));
        }
        if first_token_named(sig, "NAME").is_none() {
            return Err(err("(define ...) missing a name", at));
        }
        if let Some(ann) = annotation_child(sig) {
            check_type_annotation(ann)?;
        }
        return Ok(());
    }
    check_fn_signature(sig)
}

/// Mirror of `extract_fn_signature` for `(name typed_param* [-> T])`.
fn check_fn_signature(sig: &GrammarASTNode) -> Check {
    let mut saw_name = false;
    let mut children = sig.children.iter().peekable();
    while let Some(child) = children.next() {
        match child {
            ASTNodeOrToken::Token(t) => match t.effective_type_name() {
                "ARROW" => match children.peek() {
                    Some(ASTNodeOrToken::Node(ann)) if ann.rule_name == "type_annotation" => {
                        check_type_annotation(ann)?;
                        children.next();
                    }
                    _ => {
                        return Err(err(
                            "'->' in function signature must be followed by a type annotation",
                            (t.line, t.column),
                        ))
                    }
                },
                "NAME" => saw_name = true,
                _ => {}
            },
            ASTNodeOrToken::Node(n) if n.rule_name == "typed_param" => {
                if first_token_named(n, "NAME").is_none() {
                    return Err(err("typed_param: missing parameter name", pos(n)));
                }
                if let Some(ann) = annotation_child(n) {
                    check_type_annotation(ann)?;
                }
            }
            ASTNodeOrToken::Node(_) => {}
        }
    }
    if !saw_name {
        return Err(err("(define ...) missing function name in signature", pos(sig)));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// expr = atom | quoted | compound
// ---------------------------------------------------------------------------

fn check_expr(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    if node.rule_name != "expr" {
        return Err(err(format!("expected 'expr', got {:?}", node.rule_name), at));
    }
    let inner = first_node(node).ok_or_else(|| err("empty expr node", at))?;
    match inner.rule_name.as_str() {
        "atom" => check_atom(inner),
        "quoted" => check_named(inner, "expected NAME after '"),
        "compound" => check_compound(inner, depth + 1),
        other => Err(err(format!("unexpected expr child: {other:?}"), at)),
    }
}

/// A node that must carry a `NAME` token (`'foo`, `(quote foo)`).
fn check_named(node: &GrammarASTNode, message: &str) -> Check {
    match first_token_named(node, "NAME") {
        Some(_) => Ok(()),
        None => Err(err(message, pos(node))),
    }
}

fn check_atom(node: &GrammarASTNode) -> Check {
    let tok = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
        .ok_or_else(|| err("empty atom", pos(node)))?;
    let at = (tok.line, tok.column);
    match tok.effective_type_name() {
        "INTEGER" => tok
            .value
            .parse::<i64>()
            .map(|_| ())
            .map_err(|_| err(format!("integer literal {:?} does not fit in i64", tok.value), at)),
        "BOOL_TRUE" | "BOOL_FALSE" | "NAME" | "STRING" => Ok(()),
        "KEYWORD" if tok.value == "nil" => Ok(()),
        other => Err(err(
            format!("unexpected atom token: type={other:?} value={:?}", tok.value),
            at,
        )),
    }
}

// ---------------------------------------------------------------------------
// compound = if | let | let* | begin | lambda | quote | match | apply
// ---------------------------------------------------------------------------

fn check_compound(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    let inner = first_node(node).ok_or_else(|| err("empty compound node", at))?;
    let depth = depth + 1;
    match inner.rule_name.as_str() {
        "if_form" => {
            let n = check_exprs(inner, depth)?;
            if n != 3 {
                return Err(err("(if ...) takes exactly 3 expressions", pos(inner)));
            }
            Ok(())
        }
        "let_form" => check_let(inner, depth, "(let (...) ...) needs at least one body expression"),
        "let_star_form" => {
            check_let(inner, depth, "(let* (...) ...) needs at least one body expression")
        }
        "begin_form" => {
            if check_exprs(inner, depth)? == 0 {
                return Err(err("(begin ...) needs at least one expression", pos(inner)));
            }
            Ok(())
        }
        "lambda_form" => {
            if check_exprs(inner, depth)? == 0 {
                return Err(err(
                    "(lambda (...) ...) needs at least one body expression",
                    pos(inner),
                ));
            }
            Ok(())
        }
        "quote_form" => check_named(inner, "(quote ...) needs a name"),
        "match_form" => check_match(inner, depth),
        "apply" => {
            if check_exprs(inner, depth)? == 0 {
                return Err(err(
                    "empty application '()' — use 'nil' for the empty list",
                    pos(inner),
                ));
            }
            Ok(())
        }
        other => Err(err(format!("unexpected compound child: {other:?}"), at)),
    }
}

/// Check every `expr` child of `node` (at `depth + 1`, as the extractor
/// did) and return how many there were.
fn check_exprs(node: &GrammarASTNode, depth: usize) -> Result<usize, TwigParseError> {
    let mut n = 0;
    for e in node_children(node).filter(|c| c.rule_name == "expr") {
        check_expr(e, depth + 1)?;
        n += 1;
    }
    Ok(n)
}

/// `let` and `let*` share one shape: `binding` children and body `expr`s,
/// checked together in source order.
fn check_let(node: &GrammarASTNode, depth: usize, empty_body: &str) -> Check {
    let mut body = 0;
    for child in node_children(node) {
        match child.rule_name.as_str() {
            "binding" => check_binding(child, depth + 1)?,
            "expr" => {
                check_expr(child, depth + 1)?;
                body += 1;
            }
            _ => {}
        }
    }
    if body == 0 {
        return Err(err(empty_body, pos(node)));
    }
    Ok(())
}

fn check_binding(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    if first_token_named(node, "NAME").is_none() {
        return Err(err("malformed binding — expected (name expr)", at));
    }
    let e = node_children(node)
        .find(|c| c.rule_name == "expr")
        .ok_or_else(|| err("malformed binding — missing expression", at))?;
    check_expr(e, depth + 1)
}

fn check_match(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    let scrutinee = node_children(node)
        .find(|c| c.rule_name == "expr")
        .ok_or_else(|| err("(match ...) missing scrutinee expression", at))?;
    check_expr(scrutinee, depth + 1)?;

    let mut arms = 0;
    for arm in node_children(node).filter(|c| c.rule_name == "match_arm") {
        check_match_arm(arm, depth + 1)?;
        arms += 1;
    }
    if arms == 0 {
        return Err(err("(match ...) needs at least one arm", at));
    }
    Ok(())
}

fn check_match_arm(node: &GrammarASTNode, depth: usize) -> Check {
    let at = pos(node);
    check_depth(depth, at)?;
    let pat = node_children(node)
        .find(|c| c.rule_name == "match_pat")
        .ok_or_else(|| err("match arm missing pattern", at))?;
    if tokens_named(pat, "NAME").next().is_none() {
        let message = if has_token(pat, "LPAREN") {
            "match_pat: variant pattern missing constructor name"
        } else {
            "match_pat: expected NAME"
        };
        return Err(err(message, pos(pat)));
    }
    if check_exprs(node, depth)? == 0 {
        return Err(err("match arm missing body expression", at));
    }
    Ok(())
}
