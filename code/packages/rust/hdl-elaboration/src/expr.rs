//! Verilog expression AST → HIR `Expr` elaboration.
//!
//! The Verilog parser produces a precedence-tower of rule nodes:
//!
//! ```text
//! expression → ternary_expr → or_expr → and_expr → bit_or_expr →
//!   bit_xor_expr → bit_and_expr → equality_expr → relational_expr →
//!   shift_expr → additive_expr → multiplicative_expr → power_expr →
//!   unary_expr → primary
//! ```
//!
//! Most nodes in the tower contain a single child and simply delegate.
//! Only `primary` (and some unary/binary nodes) carry real content.
//!
//! ## Binary chains
//!
//! `additive_expr` looks like `sub_expr (OP sub_expr)*`. We fold it
//! left-associatively into a tree of `Expr::Binary` nodes.

use hdl_ir::expr::{Expr, LitValue};
use hdl_ir::types::Ty;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast::{child_nodes, child_rule, child_rules, direct_token_values,
                  first_number, is_name_token, parse_verilog_number,
                  unwrap_to_token_value};
use crate::ElaborationError;

// ---------------------------------------------------------------------------
// Top-level dispatcher
// ---------------------------------------------------------------------------

/// Maximum recursion depth for expression elaboration.
///
/// Prevents stack overflow from deeply nested attacker-supplied Verilog.
/// Legitimate expressions are never more than a few dozen levels deep.
const MAX_EXPR_DEPTH: usize = 512;

/// Elaborate a Verilog expression node into an HIR `Expr`.
pub(crate) fn elaborate_expr(node: &GrammarASTNode) -> Result<Expr, ElaborationError> {
    elaborate_expr_depth(node, 0)
}

fn elaborate_expr_depth(node: &GrammarASTNode, depth: usize) -> Result<Expr, ElaborationError> {
    if depth > MAX_EXPR_DEPTH {
        return Err(ElaborationError::InvalidExpr(
            "expression nesting depth limit exceeded".into()
        ));
    }
    let next = depth + 1;
    match node.rule_name.as_str() {
        // Rules that are just the name tower wrappers with exactly one node child:
        // unwrap them transparently.
        r @ ("expression" | "or_expr" | "and_expr" | "bit_or_expr" | "bit_xor_expr"
            | "bit_and_expr" | "equality_expr" | "relational_expr" | "shift_expr"
            | "additive_expr" | "multiplicative_expr" | "power_expr") => {
            let nodes: Vec<&GrammarASTNode> = child_nodes(node);
            if nodes.len() == 1 && node.children.len() == 1 {
                // Single node child — transparent pass-through.
                return elaborate_expr_depth(nodes[0], next);
            }
            // Multiple sub-expressions separated by operator tokens.
            elaborate_binary_chain_depth(node, r, next)
        }

        "ternary_expr" => {
            let nodes = child_nodes(node);
            if nodes.len() == 1 && node.children.len() == 1 {
                return elaborate_expr_depth(nodes[0], next);
            }
            elaborate_ternary_depth(node, next)
        }

        "unary_expr" => elaborate_unary_depth(node, next),
        "primary" => elaborate_primary_depth(node, next),
        "concatenation" => elaborate_concat_depth(node, next),
        "replication" => elaborate_replication_depth(node, next),

        _ => {
            // Unknown or wrapper rule — try transparent single-child unwrap.
            let nodes = child_nodes(node);
            if nodes.len() == 1 {
                return elaborate_expr_depth(nodes[0], next);
            }
            if node.children.len() == 1 {
                if let ASTNodeOrToken::Token(t) = &node.children[0] {
                    return primary_from_token_value(&t.value);
                }
            }
            Err(ElaborationError::InvalidExpr(format!(
                "unhandled rule '{}' with {} children", node.rule_name, node.children.len()
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// Binary chains (left-to-right fold)
// ---------------------------------------------------------------------------

fn elaborate_binary_chain_depth(node: &GrammarASTNode, _rule: &str, depth: usize) -> Result<Expr, ElaborationError> {
    let mut expr_iter = node.children.iter();
    let first = match expr_iter.next() {
        Some(ASTNodeOrToken::Node(n)) => elaborate_expr_depth(n, depth)?,
        Some(ASTNodeOrToken::Token(t)) => primary_from_token_value(&t.value)?,
        None => return Err(ElaborationError::InvalidExpr("empty binary chain".into())),
    };

    let mut result = first;
    let mut op: Option<String> = None;

    for child in expr_iter {
        match child {
            ASTNodeOrToken::Token(t) => {
                op = Some(verilog_op_to_hir(&t.value));
            }
            ASTNodeOrToken::Node(n) => {
                let rhs = elaborate_expr_depth(n, depth)?;
                let cur_op = op.take().unwrap_or_else(|| "+".to_string());
                result = Expr::Binary {
                    op: cur_op,
                    lhs: Box::new(result),
                    rhs: Box::new(rhs),
                    provenance: None,
                };
            }
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Ternary
// ---------------------------------------------------------------------------

fn elaborate_ternary_depth(node: &GrammarASTNode, depth: usize) -> Result<Expr, ElaborationError> {
    let sub_nodes: Vec<&GrammarASTNode> = child_nodes(node);
    if sub_nodes.len() < 3 {
        return Err(ElaborationError::InvalidExpr(
            "ternary_expr needs 3 sub-expressions".into()
        ));
    }
    Ok(Expr::Ternary {
        cond:      Box::new(elaborate_expr_depth(sub_nodes[0], depth)?),
        then_expr: Box::new(elaborate_expr_depth(sub_nodes[1], depth)?),
        else_expr: Box::new(elaborate_expr_depth(sub_nodes[2], depth)?),
        provenance: None,
    })
}

// ---------------------------------------------------------------------------
// Unary
// ---------------------------------------------------------------------------

fn elaborate_unary_depth(node: &GrammarASTNode, depth: usize) -> Result<Expr, ElaborationError> {
    let tokens = direct_token_values(node);
    let nodes = child_nodes(node);

    if tokens.is_empty() && nodes.len() == 1 {
        return elaborate_expr_depth(nodes[0], depth);
    }

    let unary_ops = ["+", "-", "!", "~", "&", "^", "~&", "~|", "~^"];
    let op_tok = tokens.iter().find(|t| unary_ops.contains(t));

    if let Some(op) = op_tok {
        let hir_op = verilog_unary_op_to_hir(op);
        if let Some(operand_node) = nodes.first() {
            return Ok(Expr::Unary {
                op: hir_op.to_string(),
                operand: Box::new(elaborate_expr_depth(operand_node, depth)?),
                provenance: None,
            });
        }
    }

    if let Some(n) = nodes.first() { return elaborate_expr_depth(n, depth); }

    Err(ElaborationError::InvalidExpr("unary_expr: no operand".into()))
}

// ---------------------------------------------------------------------------
// Primary
// ---------------------------------------------------------------------------

fn elaborate_primary_depth(node: &GrammarASTNode, depth: usize) -> Result<Expr, ElaborationError> {
    let subnodes = child_nodes(node);

    if subnodes.len() == 1
        && matches!(subnodes[0].rule_name.as_str(),
            "concatenation" | "replication" | "expression")
    {
        return elaborate_expr_depth(subnodes[0], depth);
    }

    if let Some(range_sel) = child_rule(node, "range_select") {
        let base_name = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if is_name_token(t) => Some(t.value.as_str()),
            _ => None,
        });
        if let Some(name) = base_name {
            return elaborate_slice(name, range_sel);
        }
    }

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) => {
                return primary_from_token_value(&t.value);
            }
            ASTNodeOrToken::Node(n) => {
                if n.rule_name == "expression" {
                    return elaborate_expr_depth(n, depth);
                }
            }
        }
    }

    if let Some(v) = unwrap_to_token_value(node) {
        return primary_from_token_value(v);
    }

    Err(ElaborationError::InvalidExpr(format!(
        "cannot elaborate primary: {:?}", direct_token_values(node)
    )))
}

fn elaborate_slice(base_name: &str, range_sel: &GrammarASTNode) -> Result<Expr, ElaborationError> {
    let exprs = child_rules(range_sel, "expression");
    match exprs.len() {
        0 => Err(ElaborationError::InvalidExpr("empty range_select".into())),
        1 => {
            // Bit-select [expr].
            let idx = first_number(exprs[0]).unwrap_or(0);
            Ok(Expr::Slice {
                base: Box::new(Expr::port_ref(base_name)),
                msb: idx,
                lsb: idx,
                provenance: None,
            })
        }
        _ => {
            // Part-select [msb:lsb].
            let msb = first_number(exprs[0]).unwrap_or(0);
            let lsb = first_number(exprs[1]).unwrap_or(0);
            Ok(Expr::Slice {
                base: Box::new(Expr::port_ref(base_name)),
                msb,
                lsb,
                provenance: None,
            })
        }
    }
}

/// Build a primary `Expr` from a raw token value (NAME or number literal).
fn primary_from_token_value(val: &str) -> Result<Expr, ElaborationError> {
    if val.is_empty() { return Err(ElaborationError::InvalidExpr("empty token".into())); }

    // Sized literal like 4'b1010 or 8'hFF.
    if val.contains('\'') {
        let num = parse_verilog_number(val).unwrap_or(0);
        return Ok(Expr::Lit {
            value: LitValue::Int(num as i64),
            ty: Ty::Bit,
            provenance: None,
        });
    }

    // Plain decimal number.
    if val.chars().all(|c| c.is_ascii_digit()) {
        let n: i64 = val.parse().unwrap_or(0);
        return Ok(Expr::Lit { value: LitValue::Int(n), ty: Ty::Bit, provenance: None });
    }

    // Identifier → PortRef (elaboration cannot distinguish port vs net at this stage;
    // hardware-vm resolves references at runtime).
    Ok(Expr::port_ref(val))
}

// ---------------------------------------------------------------------------
// Concatenation
// ---------------------------------------------------------------------------

fn elaborate_concat_depth(node: &GrammarASTNode, depth: usize) -> Result<Expr, ElaborationError> {
    let parts: Vec<Expr> = child_rules(node, "expression")
        .into_iter()
        .map(|n| elaborate_expr_depth(n, depth))
        .collect::<Result<_, _>>()?;

    if parts.is_empty() {
        return Err(ElaborationError::InvalidExpr("empty concatenation".into()));
    }
    Ok(Expr::Concat { parts, provenance: None })
}

// ---------------------------------------------------------------------------
// Replication
// ---------------------------------------------------------------------------

fn elaborate_replication_depth(node: &GrammarASTNode, depth: usize) -> Result<Expr, ElaborationError> {
    let all_exprs = child_rules(node, "expression");
    if all_exprs.is_empty() {
        return Err(ElaborationError::InvalidExpr("empty replication".into()));
    }
    let count_expr = elaborate_expr_depth(all_exprs[0], depth)?;
    let body_parts: Vec<Expr> = all_exprs[1..]
        .iter()
        .map(|n| elaborate_expr_depth(n, depth))
        .collect::<Result<_, _>>()?;

    let body = if body_parts.len() == 1 {
        body_parts.into_iter().next().unwrap()
    } else {
        Expr::Concat { parts: body_parts, provenance: None }
    };

    Ok(Expr::Replication {
        count: Box::new(count_expr),
        body: Box::new(body),
        provenance: None,
    })
}

// ---------------------------------------------------------------------------
// Operator name mapping
// ---------------------------------------------------------------------------

/// Map a Verilog binary operator token to the HIR op string.
fn verilog_op_to_hir(op: &str) -> String {
    match op {
        "+"   => "+",
        "-"   => "-",
        "*"   => "*",
        "/"   => "/",
        "%"   => "%",
        "**"  => "**",
        "&"   => "AND",
        "|"   => "OR",
        "^"   => "XOR",
        "~&"  => "NAND",
        "~|"  => "NOR",
        "~^" | "^~" => "XNOR",
        "<<"  => "<<",
        ">>"  => ">>",
        "<<<" => "<<<",
        ">>>" => ">>>",
        "=="  => "==",
        "!="  => "!=",
        "===" => "===",
        "!==" => "!==",
        "<"   => "<",
        "<="  => "<=",
        ">"   => ">",
        ">="  => ">=",
        "&&"  => "&&",
        "||"  => "||",
        _     => op,
    }.to_string()
}

/// Map a Verilog unary operator token to the HIR op string.
fn verilog_unary_op_to_hir(op: &str) -> &'static str {
    match op {
        "+" => "POS",
        "-" => "NEG",
        "!" => "LOGIC_NOT",
        "~" => "NOT",
        "&" => "AND_RED",
        "|" => "OR_RED",
        "^" => "XOR_RED",
        "~&" => "NAND_RED",
        "~|" => "NOR_RED",
        "~^" | "^~" => "XNOR_RED",
        _ => "NOT",
    }
}
