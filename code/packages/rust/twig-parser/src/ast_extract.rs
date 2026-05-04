//! Walk a `GrammarASTNode` tree → typed Twig AST.
//!
//! The grammar-driven [`parser::grammar_parser::GrammarParser`] emits one
//! node per non-terminal — wrappers like `expr` / `compound` /
//! `name_or_signature` exist to keep `code/grammars/twig.grammar`
//! readable but carry no semantic content of their own.  This module
//! lifts each meaningful subtree into one of the typed nodes from
//! [`crate::ast_nodes`].
//!
//! ## How a few key shapes lower
//!
//! | Grammar production                                           | Typed AST          |
//! |--------------------------------------------------------------|--------------------|
//! | `(define name expr)`                                         | `Define { Expr }`  |
//! | `(define (name args) body+)`                                 | `Define { Lambda }`|
//! | `'foo`                                                       | `SymLit`           |
//! | `(quote foo)`                                                | `SymLit`           |
//! | `(if c t e)`                                                 | `If`               |
//! | `(let ((x 1) (y 2)) body+)`                                  | `Let`              |
//! | `(lambda (p) body+)`                                         | `Lambda`           |
//! | `(fn arg0 arg1 ...)`                                         | `Apply`            |
//!
//! Both quote forms (`'foo` and `(quote foo)`) collapse to a single
//! `SymLit { name: "foo" }`; downstream code therefore handles symbol
//! literals in one place.  Same for `define`-sugar — the function form
//! lowers to `Define { Lambda }` so the IR compiler only ever sees the
//! lambda shape.
//!
//! ## Stack-overflow guard
//!
//! The walker is recursive (`extract_expr` recurses into compound
//! children).  To defend against pathological input that produces an
//! arbitrarily deep AST (e.g. `((((...))))` with tens of thousands of
//! opens), every recursive descent passes a `depth: usize` and bails
//! out with a `TwigParseError` once it exceeds [`MAX_AST_DEPTH`].
//! Without this guard a deeply-nested input could exhaust the OS thread
//! stack and abort the process (Rust does not catch stack overflow).

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast_nodes::{
    Apply, Begin, BoolLit, Define, Expr, Form, If, IntLit, Lambda, Let, NilLit, Program, SymLit,
    VarRef,
};
use crate::TwigParseError;

/// Maximum AST-nesting depth the extractor will descend.
///
/// 256 is far above any realistic Twig program (hand-written code is
/// single-digit-deep) while keeping a comfortable margin under the
/// typical 2 MiB OS thread stack on macOS.
pub const MAX_AST_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Token helpers
// ---------------------------------------------------------------------------

/// True iff `child` is a raw `Token` whose grammar name (or enum
/// fallback) equals `name`.
fn is_token_named(child: &ASTNodeOrToken, name: &str) -> bool {
    match child {
        ASTNodeOrToken::Token(t) => t.effective_type_name() == name,
        ASTNodeOrToken::Node(_) => false,
    }
}

/// Filter to only the nested ASTNode children, dropping bare punctuation
/// tokens (LPAREN / RPAREN / KEYWORDs / NAMEs introduced as literals).
/// Convenient when a grammar rule's children list mixes structural
/// sub-rules with literal punctuation we can ignore.
fn ast_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Best-effort source-position extraction.  Falls back to (1, 1) if the
/// node has no recorded position (rare — the GrammarParser always
/// records positions for non-empty matches).
fn pos(node: &GrammarASTNode) -> (usize, usize) {
    (node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
}

fn tok_pos(t: &Token) -> (usize, usize) {
    (t.line, t.column)
}

/// Find the first child token that matches `name` and return it.  Used
/// when the grammar rule has exactly one such token (e.g. the NAME in
/// `(quote NAME)`).
fn first_token_named<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a Token> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.effective_type_name() == name => Some(t),
        _ => None,
    })
}

/// Collect every child token of the given grammar name, in order.
fn tokens_named<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a Token> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == name => Some(t),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Depth-bounded recursion guard
// ---------------------------------------------------------------------------

fn check_depth(depth: usize, line: usize, column: usize) -> Result<(), TwigParseError> {
    if depth > MAX_AST_DEPTH {
        Err(TwigParseError {
            message: format!(
                "AST nesting exceeds MAX_AST_DEPTH ({MAX_AST_DEPTH}) — \
                 refusing to recurse further to avoid stack overflow"
            ),
            line,
            column,
        })
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Public entry: program
// ---------------------------------------------------------------------------

/// Convert a parsed `program` [`GrammarASTNode`] into a typed [`Program`].
///
/// Expects `root.rule_name == "program"` (the grammar's start symbol).
/// Each child is a `form` non-terminal which is unwrapped one level to
/// reveal either a `define` or `expr` subtree.
pub fn extract_program(root: &GrammarASTNode) -> Result<Program, TwigParseError> {
    if root.rule_name != "program" {
        let (line, column) = pos(root);
        return Err(TwigParseError {
            message: format!("expected 'program' root, got {:?}", root.rule_name),
            line,
            column,
        });
    }
    let mut forms = Vec::new();
    for form_node in ast_children(root) {
        if form_node.rule_name != "form" {
            let (line, column) = pos(form_node);
            return Err(TwigParseError {
                message: format!("expected 'form' child of program, got {:?}", form_node.rule_name),
                line,
                column,
            });
        }
        forms.push(extract_form(form_node, 0)?);
    }
    Ok(Program { forms })
}

// ---------------------------------------------------------------------------
// form = define | expr
// ---------------------------------------------------------------------------

fn extract_form(node: &GrammarASTNode, depth: usize) -> Result<Form, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let inner = ast_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| TwigParseError {
            message: "empty form node".into(),
            line,
            column,
        })?;
    match inner.rule_name.as_str() {
        "define" => Ok(Form::Define(extract_define(inner, depth + 1)?)),
        "expr" => Ok(Form::Expr(extract_expr(inner, depth + 1)?)),
        other => Err(TwigParseError {
            message: format!("unexpected form child: {other:?}"),
            line,
            column,
        }),
    }
}

// ---------------------------------------------------------------------------
// define = LPAREN "define" name_or_signature expr { expr } RPAREN
// ---------------------------------------------------------------------------

fn extract_define(node: &GrammarASTNode, depth: usize) -> Result<Define, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let children = ast_children(node);
    let sig_node = children.first().ok_or_else(|| TwigParseError {
        message: "(define ...) missing signature".into(),
        line,
        column,
    })?;
    if sig_node.rule_name != "name_or_signature" {
        return Err(TwigParseError {
            message: format!(
                "expected 'name_or_signature', got {:?}",
                sig_node.rule_name
            ),
            line,
            column,
        });
    }
    let body_exprs: Vec<Expr> = children
        .iter()
        .skip(1)
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if body_exprs.is_empty() {
        return Err(TwigParseError {
            message: "(define ...) must have a body expression".into(),
            line,
            column,
        });
    }

    // Two shapes for the signature:
    //   NAME                            — value-define
    //   LPAREN NAME { NAME } RPAREN     — function-sugar
    //
    // We distinguish them by whether the signature's children list
    // contains an LPAREN token.  Multi-NAME signatures always have
    // parens; single-NAME-without-parens signatures don't.
    let name_toks = tokens_named(sig_node, "NAME");
    if name_toks.is_empty() {
        return Err(TwigParseError {
            message: "(define ...) missing a name".into(),
            line,
            column,
        });
    }
    let has_paren = sig_node
        .children
        .iter()
        .any(|c| is_token_named(c, "LPAREN"));

    if !has_paren {
        // Plain (define name expr) — exactly one body expression.
        if body_exprs.len() != 1 {
            return Err(TwigParseError {
                message: "(define name expr) takes exactly one body expression — \
                          use (define (name args...) body+) for multi-expression bodies"
                    .into(),
                line,
                column,
            });
        }
        let expr = body_exprs.into_iter().next().unwrap();
        return Ok(Define {
            name: name_toks[0].value.clone(),
            expr,
            line,
            column,
        });
    }

    // Function-sugar: (define (name args...) body+)
    let fn_name = name_toks[0].value.clone();
    let params: Vec<String> = name_toks[1..].iter().map(|t| t.value.clone()).collect();
    let (sig_line, sig_col) = tok_pos(name_toks[0]);
    let lam = Lambda {
        params,
        body: body_exprs,
        line: sig_line,
        column: sig_col,
    };
    Ok(Define {
        name: fn_name,
        expr: Expr::Lambda(lam),
        line,
        column,
    })
}

// ---------------------------------------------------------------------------
// expr = atom | quoted | compound
// ---------------------------------------------------------------------------

fn extract_expr(node: &GrammarASTNode, depth: usize) -> Result<Expr, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    if node.rule_name != "expr" {
        return Err(TwigParseError {
            message: format!("expected 'expr', got {:?}", node.rule_name),
            line,
            column,
        });
    }
    let inner = ast_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| TwigParseError {
            message: "empty expr node".into(),
            line,
            column,
        })?;
    match inner.rule_name.as_str() {
        "atom" => extract_atom(inner),
        "quoted" => Ok(Expr::SymLit(extract_quoted(inner)?)),
        "compound" => extract_compound(inner, depth + 1),
        other => Err(TwigParseError {
            message: format!("unexpected expr child: {other:?}"),
            line,
            column,
        }),
    }
}

// ---------------------------------------------------------------------------
// atom = INTEGER | BOOL_TRUE | BOOL_FALSE | "nil" | NAME
// ---------------------------------------------------------------------------

fn extract_atom(node: &GrammarASTNode) -> Result<Expr, TwigParseError> {
    let (line, column) = pos(node);
    // Atom children are bare tokens, not nested nodes.
    let tok = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
        .ok_or_else(|| TwigParseError {
            message: "empty atom".into(),
            line,
            column,
        })?;
    let (l, c) = tok_pos(tok);
    let name = tok.effective_type_name();
    match name {
        "INTEGER" => {
            let value: i64 = tok.value.parse().map_err(|_| TwigParseError {
                message: format!("integer literal {:?} does not fit in i64", tok.value),
                line: l,
                column: c,
            })?;
            Ok(Expr::IntLit(IntLit { value, line: l, column: c }))
        }
        "BOOL_TRUE" => Ok(Expr::BoolLit(BoolLit { value: true, line: l, column: c })),
        "BOOL_FALSE" => Ok(Expr::BoolLit(BoolLit { value: false, line: l, column: c })),
        "KEYWORD" if tok.value == "nil" => Ok(Expr::NilLit(NilLit { line: l, column: c })),
        "NAME" => Ok(Expr::VarRef(VarRef { name: tok.value.clone(), line: l, column: c })),
        other => Err(TwigParseError {
            message: format!(
                "unexpected atom token: type={other:?} value={:?}",
                tok.value
            ),
            line: l,
            column: c,
        }),
    }
}

// ---------------------------------------------------------------------------
// quoted = QUOTE NAME
// ---------------------------------------------------------------------------

fn extract_quoted(node: &GrammarASTNode) -> Result<SymLit, TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "expected NAME after '".into(),
        line,
        column,
    })?;
    Ok(SymLit { name: name_tok.value.clone(), line, column })
}

// ---------------------------------------------------------------------------
// compound = if_form | let_form | begin_form | lambda_form | quote_form | apply
// ---------------------------------------------------------------------------

fn extract_compound(node: &GrammarASTNode, depth: usize) -> Result<Expr, TwigParseError> {
    let (line, column) = pos(node);
    check_depth(depth, line, column)?;
    let inner = ast_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| TwigParseError {
            message: "empty compound node".into(),
            line,
            column,
        })?;
    match inner.rule_name.as_str() {
        "if_form" => Ok(Expr::If(extract_if(inner, depth + 1)?)),
        "let_form" => Ok(Expr::Let(extract_let(inner, depth + 1)?)),
        "begin_form" => Ok(Expr::Begin(extract_begin(inner, depth + 1)?)),
        "lambda_form" => Ok(Expr::Lambda(extract_lambda(inner, depth + 1)?)),
        "quote_form" => Ok(Expr::SymLit(extract_quote_form(inner)?)),
        "apply" => Ok(Expr::Apply(extract_apply(inner, depth + 1)?)),
        other => Err(TwigParseError {
            message: format!("unexpected compound child: {other:?}"),
            line,
            column,
        }),
    }
}

fn extract_if(node: &GrammarASTNode, depth: usize) -> Result<If, TwigParseError> {
    let (line, column) = pos(node);
    let exprs: Vec<Expr> = ast_children(node)
        .into_iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if exprs.len() != 3 {
        return Err(TwigParseError {
            message: "(if ...) takes exactly 3 expressions".into(),
            line,
            column,
        });
    }
    let mut it = exprs.into_iter();
    let cond = Box::new(it.next().unwrap());
    let then_branch = Box::new(it.next().unwrap());
    let else_branch = Box::new(it.next().unwrap());
    Ok(If { cond, then_branch, else_branch, line, column })
}

fn extract_let(node: &GrammarASTNode, depth: usize) -> Result<Let, TwigParseError> {
    let (line, column) = pos(node);
    let mut bindings: Vec<(String, Expr)> = Vec::new();
    let mut body: Vec<Expr> = Vec::new();
    for child in ast_children(node) {
        match child.rule_name.as_str() {
            "binding" => bindings.push(extract_binding(child, depth + 1)?),
            "expr" => body.push(extract_expr(child, depth + 1)?),
            _ => {}
        }
    }
    if body.is_empty() {
        return Err(TwigParseError {
            message: "(let (...) ...) needs at least one body expression".into(),
            line,
            column,
        });
    }
    Ok(Let { bindings, body, line, column })
}

fn extract_binding(
    node: &GrammarASTNode,
    depth: usize,
) -> Result<(String, Expr), TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "malformed binding — expected (name expr)".into(),
        line,
        column,
    })?;
    let expr_node = ast_children(node)
        .into_iter()
        .find(|c| c.rule_name == "expr")
        .ok_or_else(|| TwigParseError {
            message: "malformed binding — missing expression".into(),
            line,
            column,
        })?;
    let expr = extract_expr(expr_node, depth + 1)?;
    Ok((name_tok.value.clone(), expr))
}

fn extract_begin(node: &GrammarASTNode, depth: usize) -> Result<Begin, TwigParseError> {
    let (line, column) = pos(node);
    let exprs: Vec<Expr> = ast_children(node)
        .into_iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if exprs.is_empty() {
        return Err(TwigParseError {
            message: "(begin ...) needs at least one expression".into(),
            line,
            column,
        });
    }
    Ok(Begin { exprs, line, column })
}

fn extract_lambda(node: &GrammarASTNode, depth: usize) -> Result<Lambda, TwigParseError> {
    let (line, column) = pos(node);
    // Walk children left-to-right: collect NAMEs (params) before the
    // first nested `expr` ASTNode, then collect `expr` ASTNodes as body.
    let mut params: Vec<String> = Vec::new();
    let mut body: Vec<Expr> = Vec::new();
    let mut seen_first_expr = false;
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) if n.rule_name == "expr" => {
                body.push(extract_expr(n, depth + 1)?);
                seen_first_expr = true;
            }
            ASTNodeOrToken::Token(t)
                if !seen_first_expr && t.effective_type_name() == "NAME" =>
            {
                params.push(t.value.clone());
            }
            _ => {}
        }
    }
    if body.is_empty() {
        return Err(TwigParseError {
            message: "(lambda (...) ...) needs at least one body expression".into(),
            line,
            column,
        });
    }
    Ok(Lambda { params, body, line, column })
}

fn extract_quote_form(node: &GrammarASTNode) -> Result<SymLit, TwigParseError> {
    let (line, column) = pos(node);
    let name_tok = first_token_named(node, "NAME").ok_or_else(|| TwigParseError {
        message: "(quote ...) needs a name".into(),
        line,
        column,
    })?;
    Ok(SymLit { name: name_tok.value.clone(), line, column })
}

fn extract_apply(node: &GrammarASTNode, depth: usize) -> Result<Apply, TwigParseError> {
    let (line, column) = pos(node);
    let exprs: Vec<Expr> = ast_children(node)
        .into_iter()
        .filter(|c| c.rule_name == "expr")
        .map(|c| extract_expr(c, depth + 1))
        .collect::<Result<_, _>>()?;
    if exprs.is_empty() {
        return Err(TwigParseError {
            message: "empty application '()' — use 'nil' for the empty list".into(),
            line,
            column,
        });
    }
    let mut it = exprs.into_iter();
    let fn_expr = Box::new(it.next().unwrap());
    let args: Vec<Expr> = it.collect();
    Ok(Apply { fn_expr, args, line, column })
}

