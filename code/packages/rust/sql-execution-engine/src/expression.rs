//! Expression evaluator — evaluate AST expression nodes against a row.
//!
//! The SQL grammar produces a recursive tree of expression nodes.
//! This module walks that tree and computes a [`SqlValue`].
//!
//! # Expression grammar (abbreviated)
//!
//! ```text
//! expr            = or_expr
//! or_expr         = and_expr { "OR" and_expr }
//! and_expr        = not_expr { "AND" not_expr }
//! not_expr        = [ "NOT" ] comparison
//! comparison      = additive { cmp_op additive }
//!                 | additive "BETWEEN" additive "AND" additive
//!                 | additive "IN" "(" value_list ")"
//!                 | additive "LIKE" additive
//!                 | additive "IS" "NULL"
//!                 | additive "IS" "NOT" "NULL"
//! additive        = multiplicative { ("+" | "-") multiplicative }
//! multiplicative  = unary { ("*" | "/" | "%") unary }
//! unary           = [ "-" ] primary
//! primary         = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
//!                 | column_ref | function_call | "(" expr ")"
//! column_ref      = NAME [ "." NAME ]
//! function_call   = NAME "(" ( "*" | expr { "," expr } ) ")"
//! ```
//!
//! # SQL NULL
//!
//! `None` represents SQL NULL. NULL propagates through arithmetic and
//! comparisons (any operation involving NULL yields NULL).
//! `IS NULL` and `IS NOT NULL` are the only tests for NULL.

use std::collections::HashMap;

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::data_source::{SqlPrimitive, SqlValue};
use crate::errors::ExecutionError;

/// Evaluate an AST expression node against a row context.
///
/// # Arguments
///
/// * `node`    — an [`ASTNodeOrToken`] from the grammar-driven parser
/// * `row_ctx` — the current row's values, keyed by column name
///              and optionally by `"table.column"` qualified names
///
/// # Returns
///
/// The computed [`SqlValue`] — `None` for NULL, or `Some(SqlPrimitive)`.
///
/// # Errors
///
/// Returns [`ExecutionError::ColumnNotFound`] if a column reference
/// cannot be resolved in `row_ctx`.
pub fn eval_expr(
    node: &ASTNodeOrToken,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    match node {
        ASTNodeOrToken::Token(tok) => eval_token(tok, row_ctx),
        ASTNodeOrToken::Node(ast) => eval_node(ast, row_ctx),
    }
}

/// Evaluate a grammar AST node.
pub fn eval_node(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    match node.rule_name.as_str() {
        "expr" | "statement" | "program" => {
            eval_first_child(node, row_ctx)
        }
        "or_expr" => eval_or(node, row_ctx),
        "and_expr" => eval_and(node, row_ctx),
        "not_expr" => eval_not(node, row_ctx),
        "comparison" => eval_comparison(node, row_ctx),
        "additive" => eval_additive(node, row_ctx),
        "multiplicative" => eval_multiplicative(node, row_ctx),
        "unary" => eval_unary(node, row_ctx),
        "primary" => eval_primary(node, row_ctx),
        "column_ref" => eval_column_ref(node, row_ctx),
        "function_call" => eval_function_call(node, row_ctx),
        _ => {
            // Pass-through for any intermediate nodes
            eval_first_child(node, row_ctx)
        }
    }
}

fn eval_first_child(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    if let Some(child) = node.children.first() {
        eval_expr(child, row_ctx)
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Token evaluation
// ---------------------------------------------------------------------------

fn eval_token(
    token: &Token,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    let type_name = token.effective_type_name();
    let value = &token.value;

    match type_name {
        "NUMBER" => {
            if value.contains('.') {
                Ok(Some(SqlPrimitive::Float(value.parse::<f64>().unwrap_or(0.0))))
            } else {
                Ok(Some(SqlPrimitive::Int(value.parse::<i64>().unwrap_or(0))))
            }
        }
        "STRING" => {
            // The lexer already strips surrounding quotes
            Ok(Some(SqlPrimitive::Text(value.clone())))
        }
        "KEYWORD" => match value.to_uppercase().as_str() {
            "NULL" => Ok(None),
            "TRUE" => Ok(Some(SqlPrimitive::Bool(true))),
            "FALSE" => Ok(Some(SqlPrimitive::Bool(false))),
            _ => Ok(Some(SqlPrimitive::Text(value.clone()))),
        },
        "NAME" => resolve_column(value, row_ctx),
        _ => Ok(Some(SqlPrimitive::Text(value.clone()))),
    }
}

// ---------------------------------------------------------------------------
// Boolean operators
// ---------------------------------------------------------------------------

fn eval_or(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    // Grammar: and_expr { "OR" and_expr }
    let mut result = eval_expr(&node.children[0], row_ctx)?;
    let mut i = 1;
    while i < node.children.len() {
        i += 1; // skip "OR" token
        if i >= node.children.len() { break; }
        let right = eval_expr(&node.children[i], row_ctx)?;
        i += 1;
        // SQL three-valued logic
        result = match (&result, &right) {
            (Some(SqlPrimitive::Bool(true)), _) | (_, Some(SqlPrimitive::Bool(true))) => {
                Some(SqlPrimitive::Bool(true))
            }
            (None, _) | (_, None) => None,
            _ => Some(SqlPrimitive::Bool(false)),
        };
    }
    Ok(result)
}

fn eval_and(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    // Grammar: not_expr { "AND" not_expr }
    let mut result = eval_expr(&node.children[0], row_ctx)?;
    let mut i = 1;
    while i < node.children.len() {
        i += 1; // skip "AND" token
        if i >= node.children.len() { break; }
        let right = eval_expr(&node.children[i], row_ctx)?;
        i += 1;
        result = match (&result, &right) {
            (Some(SqlPrimitive::Bool(false)), _) | (_, Some(SqlPrimitive::Bool(false))) => {
                Some(SqlPrimitive::Bool(false))
            }
            (None, _) | (_, None) => None,
            _ => {
                let l = is_truthy(&result);
                let r = is_truthy(&right);
                Some(SqlPrimitive::Bool(l && r))
            }
        };
    }
    Ok(result)
}

fn eval_not(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    // Grammar: [ "NOT" ] comparison
    if !node.children.is_empty() && is_keyword_token(&node.children[0], "NOT") {
        let val = eval_expr(&node.children[1], row_ctx)?;
        Ok(match val {
            None => None,
            Some(v) => Some(SqlPrimitive::Bool(!is_truthy(&Some(v)))),
        })
    } else {
        eval_first_child(node, row_ctx)
    }
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

fn eval_comparison(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    if node.children.len() == 1 {
        return eval_first_child(node, row_ctx);
    }

    let left = eval_expr(&node.children[0], row_ctx)?;
    let second_kw = keyword_value(&node.children[1]);

    // IS NULL / IS NOT NULL
    if second_kw == "IS" {
        let third_kw = keyword_value(&node.children[2]);
        return Ok(if third_kw == "NOT" {
            Some(SqlPrimitive::Bool(left.is_some()))
        } else {
            Some(SqlPrimitive::Bool(left.is_none()))
        });
    }

    // BETWEEN
    if second_kw == "BETWEEN" {
        let low  = eval_expr(&node.children[2], row_ctx)?;
        let high = eval_expr(&node.children[4], row_ctx)?; // children[3] is AND
        return match (&left, &low, &high) {
            (None, _, _) | (_, None, _) | (_, _, None) => Ok(None),
            _ => Ok(Some(SqlPrimitive::Bool(
                compare_values(&low, &left) != std::cmp::Ordering::Greater
                    && compare_values(&left, &high) != std::cmp::Ordering::Greater,
            ))),
        };
    }

    // IN (value_list)
    if second_kw == "IN" {
        // children: left IN ( value_list )
        let value_list = &node.children[3];
        let values = eval_value_list(value_list, row_ctx)?;
        return match &left {
            None => Ok(None),
            Some(_) => Ok(Some(SqlPrimitive::Bool(values.contains(&left)))),
        };
    }

    // LIKE
    if second_kw == "LIKE" {
        let pattern = eval_expr(&node.children[2], row_ctx)?;
        return match (&left, &pattern) {
            (None, _) | (_, None) => Ok(None),
            (Some(SqlPrimitive::Text(val)), Some(SqlPrimitive::Text(pat))) => {
                Ok(Some(SqlPrimitive::Bool(like_match(val, pat))))
            }
            _ => Ok(Some(SqlPrimitive::Bool(false))),
        };
    }

    // NOT IN / NOT LIKE / NOT BETWEEN
    if second_kw == "NOT" {
        let third_kw = keyword_value(&node.children[2]);
        match third_kw.as_str() {
            "BETWEEN" => {
                let low  = eval_expr(&node.children[3], row_ctx)?;
                let high = eval_expr(&node.children[5], row_ctx)?;
                return match (&left, &low, &high) {
                    (None, _, _) | (_, None, _) | (_, _, None) => Ok(None),
                    _ => Ok(Some(SqlPrimitive::Bool(
                        !(compare_values(&low, &left) != std::cmp::Ordering::Greater
                            && compare_values(&left, &high) != std::cmp::Ordering::Greater),
                    ))),
                };
            }
            "IN" => {
                let value_list = &node.children[4];
                let values = eval_value_list(value_list, row_ctx)?;
                return match &left {
                    None => Ok(None),
                    Some(_) => Ok(Some(SqlPrimitive::Bool(!values.contains(&left)))),
                };
            }
            "LIKE" => {
                let pattern = eval_expr(&node.children[3], row_ctx)?;
                return match (&left, &pattern) {
                    (None, _) | (_, None) => Ok(None),
                    (Some(SqlPrimitive::Text(val)), Some(SqlPrimitive::Text(pat))) => {
                        Ok(Some(SqlPrimitive::Bool(!like_match(val, pat))))
                    }
                    _ => Ok(Some(SqlPrimitive::Bool(true))),
                };
            }
            _ => {}
        }
    }

    // Standard binary operator: left cmp_op right
    let op = get_op_string(&node.children[1]);
    let right = eval_expr(&node.children[2], row_ctx)?;
    match (&left, &right) {
        (None, _) | (_, None) => Ok(None),
        _ => Ok(Some(SqlPrimitive::Bool(apply_cmp(&op, &left, &right)))),
    }
}

fn apply_cmp(op: &str, left: &SqlValue, right: &SqlValue) -> bool {
    let ord = compare_values(left, right);
    match op {
        "=" => ord == std::cmp::Ordering::Equal,
        "!=" | "<>" => ord != std::cmp::Ordering::Equal,
        "<" => ord == std::cmp::Ordering::Less,
        ">" => ord == std::cmp::Ordering::Greater,
        "<=" => ord != std::cmp::Ordering::Greater,
        ">=" => ord != std::cmp::Ordering::Less,
        _ => false,
    }
}

/// Compare two SqlValues for ordering.
fn compare_values(a: &SqlValue, b: &SqlValue) -> std::cmp::Ordering {
    match (a, b) {
        (Some(SqlPrimitive::Int(x)), Some(SqlPrimitive::Int(y))) => x.cmp(y),
        (Some(SqlPrimitive::Float(x)), Some(SqlPrimitive::Float(y))) => {
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(SqlPrimitive::Int(x)), Some(SqlPrimitive::Float(y))) => {
            (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(SqlPrimitive::Float(x)), Some(SqlPrimitive::Int(y))) => {
            x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(SqlPrimitive::Text(x)), Some(SqlPrimitive::Text(y))) => {
            x.to_lowercase().cmp(&y.to_lowercase())
        }
        (Some(SqlPrimitive::Bool(x)), Some(SqlPrimitive::Bool(y))) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

fn eval_value_list(
    node: &ASTNodeOrToken,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<Vec<SqlValue>, ExecutionError> {
    match node {
        ASTNodeOrToken::Token(tok) => Ok(vec![eval_token(tok, row_ctx)?]),
        ASTNodeOrToken::Node(ast) => {
            let mut values = Vec::new();
            for child in &ast.children {
                if let ASTNodeOrToken::Token(tok) = child {
                    if tok.value == "," { continue; }
                }
                values.push(eval_expr(child, row_ctx)?);
            }
            Ok(values)
        }
    }
}

/// SQL LIKE matching with `%` wildcard support.
fn like_match(value: &str, pattern: &str) -> bool {
    // Convert SQL LIKE pattern to a simple matching algorithm.
    // % matches any sequence of characters.
    let value_lower = value.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    // Split by % and do sequential matching.
    let parts: Vec<&str> = pattern_lower.split('%').collect();
    if parts.is_empty() { return true; }

    let mut remaining = value_lower.as_str();

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() { continue; }
        if i == 0 {
            // First part must be a prefix
            if !remaining.starts_with(part) { return false; }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            // Last part must be a suffix
            if !remaining.ends_with(part) { return false; }
        } else {
            // Middle parts must appear somewhere
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }
    }

    // If no trailing %, remaining must be empty (or last part handled it)
    if !pattern_lower.ends_with('%') {
        let last = parts.last().unwrap();
        if !last.is_empty() {
            // Already handled above
        } else {
            // Pattern ends with % — anything matches
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

fn eval_additive(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    let mut result = eval_expr(&node.children[0], row_ctx)?;
    let mut i = 1;
    while i < node.children.len() {
        let op = token_value(&node.children[i]);
        i += 1;
        let right = eval_expr(&node.children[i], row_ctx)?;
        i += 1;
        result = match (result, right) {
            (None, _) | (_, None) => None,
            (Some(SqlPrimitive::Int(l)), Some(SqlPrimitive::Int(r))) => {
                if op == "+" { Some(SqlPrimitive::Int(l + r)) }
                else { Some(SqlPrimitive::Int(l - r)) }
            }
            (l, r) => {
                let lf = sql_to_float(&Some(l.unwrap()));
                let rf = sql_to_float(&Some(r.unwrap()));
                match (lf, rf) {
                    (Some(lv), Some(rv)) => {
                        let res = if op == "+" { lv + rv } else { lv - rv };
                        Some(SqlPrimitive::Float(res))
                    }
                    _ => None,
                }
            }
        };
    }
    Ok(result)
}

fn eval_multiplicative(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    let mut result = eval_expr(&node.children[0], row_ctx)?;
    let mut i = 1;
    while i < node.children.len() {
        let op = token_value(&node.children[i]);
        i += 1;
        let right = eval_expr(&node.children[i], row_ctx)?;
        i += 1;
        result = match (result, right) {
            (None, _) | (_, None) => None,
            (Some(l), Some(r)) => {
                let lf = sql_to_float(&Some(l));
                let rf = sql_to_float(&Some(r));
                match (lf, rf) {
                    (Some(lv), Some(rv)) => match op.as_str() {
                        "*" => {
                            let res = lv * rv;
                            if res.fract() == 0.0 { Some(SqlPrimitive::Int(res as i64)) }
                            else { Some(SqlPrimitive::Float(res)) }
                        }
                        "/" => {
                            if rv == 0.0 { None }
                            else {
                                let res = lv / rv;
                                if res.fract() == 0.0 { Some(SqlPrimitive::Int(res as i64)) }
                                else { Some(SqlPrimitive::Float(res)) }
                            }
                        }
                        "%" => {
                            if rv == 0.0 { None }
                            else { Some(SqlPrimitive::Float(lv % rv)) }
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
        };
    }
    Ok(result)
}

fn eval_unary(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    if !node.children.is_empty() {
        if let ASTNodeOrToken::Token(tok) = &node.children[0] {
            if tok.value == "-" {
                let val = eval_expr(&node.children[1], row_ctx)?;
                return Ok(match val {
                    None => None,
                    Some(SqlPrimitive::Int(i)) => Some(SqlPrimitive::Int(-i)),
                    Some(SqlPrimitive::Float(f)) => Some(SqlPrimitive::Float(-f)),
                    other => other,
                });
            }
        }
    }
    eval_first_child(node, row_ctx)
}

// ---------------------------------------------------------------------------
// Primary
// ---------------------------------------------------------------------------

fn eval_primary(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    if node.children.is_empty() {
        return Ok(None);
    }
    match &node.children[0] {
        ASTNodeOrToken::Token(tok) => {
            if tok.value == "(" {
                // Parenthesized expression
                eval_expr(&node.children[1], row_ctx)
            } else {
                eval_token(tok, row_ctx)
            }
        }
        ASTNodeOrToken::Node(_) => eval_first_child(node, row_ctx),
    }
}

// ---------------------------------------------------------------------------
// Column reference
// ---------------------------------------------------------------------------

fn eval_column_ref(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    if node.children.len() == 1 {
        let name = token_value(&node.children[0]);
        resolve_column(&name, row_ctx)
    } else if node.children.len() >= 3 {
        // table.column
        let table = token_value(&node.children[0]);
        let col = token_value(&node.children[2]);
        let qualified = format!("{table}.{col}");
        if let Some(val) = row_ctx.get(&qualified) {
            return Ok(val.clone());
        }
        resolve_column(&col, row_ctx)
    } else {
        Ok(None)
    }
}

fn resolve_column(
    name: &str,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    if let Some(val) = row_ctx.get(name) {
        return Ok(val.clone());
    }
    let name_lower = name.to_lowercase();
    for (key, val) in row_ctx {
        if key.to_lowercase() == name_lower {
            return Ok(val.clone());
        }
    }
    Err(ExecutionError::ColumnNotFound(name.to_string()))
}

// ---------------------------------------------------------------------------
// Function call
// ---------------------------------------------------------------------------

fn eval_function_call(
    node: &GrammarASTNode,
    row_ctx: &HashMap<String, SqlValue>,
) -> Result<SqlValue, ExecutionError> {
    let func_name = token_value(&node.children[0]).to_uppercase();

    // Check for pre-computed aggregate value
    let agg_key = make_agg_key(&func_name, &node.children[2..node.children.len().saturating_sub(1)]);
    if let Some(val) = row_ctx.get(&agg_key) {
        return Ok(val.clone());
    }

    // Scalar functions
    let arg_val = if node.children.len() > 2 {
        eval_expr(&node.children[2], row_ctx)?
    } else {
        None
    };

    Ok(match func_name.as_str() {
        "UPPER" => match arg_val {
            Some(SqlPrimitive::Text(s)) => Some(SqlPrimitive::Text(s.to_uppercase())),
            other => other,
        },
        "LOWER" => match arg_val {
            Some(SqlPrimitive::Text(s)) => Some(SqlPrimitive::Text(s.to_lowercase())),
            other => other,
        },
        "LENGTH" => match &arg_val {
            Some(SqlPrimitive::Text(s)) => Some(SqlPrimitive::Int(s.len() as i64)),
            _ => None,
        },
        "ABS" => match arg_val {
            Some(SqlPrimitive::Int(i)) => Some(SqlPrimitive::Int(i.abs())),
            Some(SqlPrimitive::Float(f)) => Some(SqlPrimitive::Float(f.abs())),
            other => other,
        },
        _ => None,
    })
}

fn make_agg_key(func_name: &str, arg_children: &[ASTNodeOrToken]) -> String {
    let arg: String = arg_children.iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(tok) if tok.value != "," => Some(tok.value.clone()),
            ASTNodeOrToken::Node(n) => Some(node_text_ast(n)),
            _ => None,
        })
        .collect();
    format!("_agg_{func_name}({arg})")
}

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

fn is_keyword_token(node: &ASTNodeOrToken, value: &str) -> bool {
    if let ASTNodeOrToken::Token(tok) = node {
        tok.value.to_uppercase() == value.to_uppercase()
    } else {
        false
    }
}

fn keyword_value(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.to_uppercase(),
        ASTNodeOrToken::Node(n) => {
            if let Some(first) = n.children.first() {
                keyword_value(first)
            } else {
                String::new()
            }
        }
    }
}

fn token_value(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.clone(),
        ASTNodeOrToken::Node(n) => {
            if let Some(first) = n.children.first() {
                token_value(first)
            } else {
                String::new()
            }
        }
    }
}

fn get_op_string(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.clone(),
        ASTNodeOrToken::Node(n) => {
            n.children.iter()
                .filter_map(|c| {
                    if let ASTNodeOrToken::Token(tok) = c { Some(tok.value.clone()) }
                    else { None }
                })
                .collect::<Vec<_>>()
                .join("")
        }
    }
}

pub fn node_text(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.clone(),
        ASTNodeOrToken::Node(n) => node_text_ast(n),
    }
}

fn node_text_ast(node: &GrammarASTNode) -> String {
    node.children.iter().map(node_text).collect()
}

fn is_truthy(val: &SqlValue) -> bool {
    match val {
        None => false,
        Some(SqlPrimitive::Bool(b)) => *b,
        Some(SqlPrimitive::Int(i)) => *i != 0,
        Some(SqlPrimitive::Float(f)) => *f != 0.0,
        Some(SqlPrimitive::Text(s)) => !s.is_empty(),
    }
}

fn sql_to_float(val: &SqlValue) -> Option<f64> {
    match val {
        Some(SqlPrimitive::Int(i)) => Some(*i as f64),
        Some(SqlPrimitive::Float(f)) => Some(*f),
        _ => None,
    }
}
