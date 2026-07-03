//! Elaboration of a `module_declaration` AST node into an HIR `Module`.
//!
//! ## Three-step pass
//!
//! 1. **Name** — the NAME token right after `module` keyword.
//! 2. **Ports** — iterate `port` nodes in the `port_list`, extracting
//!    direction, optional range `[msb:lsb]`, and signal name(s).
//! 3. **Assigns** — find every `continuous_assign` → `assignment` pair and
//!    elaborate the LHS and RHS into `ContAssign` structs.
//!
//! ## Port inheritance
//!
//! ANSI Verilog 2001 allows port direction to propagate to subsequent ports:
//! `(input a, b, output c)` means both `a` and `b` are inputs. We track the
//! last seen direction and re-use it if a port has no explicit direction.

use hdl_ir::expr::Expr;
use hdl_ir::module::{ContAssign, Direction, Level, Module, Port};
use hdl_ir::types::Ty;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::ast::{child_rule, child_rules, direct_token_values,
                  first_number, is_name_token};
use crate::expr::elaborate_expr;
use crate::ElaborationError;

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// Elaborate a `module_declaration` AST node into an HIR `Module`.
pub(crate) fn elaborate_module(decl: &GrammarASTNode) -> Result<Module, ElaborationError> {
    let name = module_name(decl)?;
    let ports = elaborate_ports(decl)?;
    let cont_assigns = elaborate_cont_assigns(decl)?;

    Ok(Module {
        name,
        ports,
        cont_assigns,
        level: Level::Structural,
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Name
// ---------------------------------------------------------------------------

fn module_name(decl: &GrammarASTNode) -> Result<String, ElaborationError> {
    // The module name is the first NAME token after the `module` keyword.
    let mut saw_module = false;
    for child in &decl.children {
        match child {
            ASTNodeOrToken::Token(t) if t.value == "module" => { saw_module = true; }
            ASTNodeOrToken::Token(t) if saw_module && is_name_token(t) => {
                return Ok(t.value.clone());
            }
            _ => {}
        }
    }
    Err(ElaborationError::InvalidModule("module_declaration: no module name".into()))
}

// ---------------------------------------------------------------------------
// Ports
// ---------------------------------------------------------------------------

fn elaborate_ports(decl: &GrammarASTNode) -> Result<Vec<Port>, ElaborationError> {
    let Some(port_list) = child_rule(decl, "port_list") else {
        return Ok(vec![]);
    };
    let mut ports = Vec::new();
    let mut prev_dir: Option<Direction> = None;

    for child in &port_list.children {
        if let ASTNodeOrToken::Node(n) = child {
            if n.rule_name == "port" {
                ports.extend(elaborate_port(n, &mut prev_dir)?);
            }
        }
    }
    Ok(ports)
}

fn elaborate_port(
    node: &GrammarASTNode,
    prev_dir: &mut Option<Direction>,
) -> Result<Vec<Port>, ElaborationError> {
    let dir = port_direction(node).or_else(|| prev_dir.clone());
    if let Some(d) = &dir { *prev_dir = Some(d.clone()); }
    let dir = dir.unwrap_or(Direction::In);
    let ty = port_type(node);
    let names = port_names(node);
    if names.is_empty() {
        return Err(ElaborationError::InvalidPort("port has no name".into()));
    }
    Ok(names.into_iter().map(|name| Port {
        name, ty: ty.clone(), direction: dir.clone(), provenance: None,
    }).collect())
}

fn port_direction(node: &GrammarASTNode) -> Option<Direction> {
    if let Some(pd) = child_rule(node, "port_direction") {
        if let Some(t) = direct_token_values(pd).first() {
            return token_to_direction(t);
        }
    }
    // Also scan direct tokens for keywords.
    for t in direct_token_values(node) {
        if let Some(d) = token_to_direction(t) { return Some(d); }
    }
    None
}

fn token_to_direction(t: &str) -> Option<Direction> {
    match t {
        "input"  => Some(Direction::In),
        "output" => Some(Direction::Out),
        "inout"  => Some(Direction::Inout),
        _ => None,
    }
}

fn port_type(node: &GrammarASTNode) -> Ty {
    let Some(range) = child_rule(node, "range") else { return Ty::Bit; };
    let exprs = child_rules(range, "expression");
    if exprs.len() < 2 { return Ty::Bit; }
    let msb = first_number(exprs[0]).unwrap_or(0);
    let lsb = first_number(exprs[1]).unwrap_or(0);
    // Use checked arithmetic to avoid u32 overflow on extreme indices.
    let width = if msb >= lsb {
        msb.checked_sub(lsb).and_then(|d| d.checked_add(1))
    } else {
        lsb.checked_sub(msb).and_then(|d| d.checked_add(1))
    }.unwrap_or(1); // Saturate to 1-bit on overflow (e.g., [u32::MAX:0])
    Ty::vec(width)
}

fn port_names(node: &GrammarASTNode) -> Vec<String> {
    let mut names = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if is_name_token(t) => {
                names.push(t.value.clone());
            }
            ASTNodeOrToken::Node(n)
                if matches!(n.rule_name.as_str(), "range" | "port_direction" | "net_type") => {}
            ASTNodeOrToken::Node(n) if n.rule_name == "name_list" => {
                for c in &n.children {
                    if let ASTNodeOrToken::Token(t) = c {
                        if is_name_token(t) { names.push(t.value.clone()); }
                    }
                }
            }
            _ => {}
        }
    }
    names
}

// ---------------------------------------------------------------------------
// Continuous assignments
// ---------------------------------------------------------------------------

fn elaborate_cont_assigns(decl: &GrammarASTNode) -> Result<Vec<ContAssign>, ElaborationError> {
    // Use direct-child lookup (child_rules) rather than deep find_all to avoid
    // quadratic traversal when module_item subtrees are large.
    let mut assigns = Vec::new();
    for item in child_rules(decl, "module_item") {
        for ca in child_rules(item, "continuous_assign") {
            for asn in child_rules(ca, "assignment") {
                assigns.push(elaborate_assignment(asn)?);
            }
        }
    }
    Ok(assigns)
}

fn elaborate_assignment(node: &GrammarASTNode) -> Result<ContAssign, ElaborationError> {
    let lv = child_rule(node, "lvalue")
        .ok_or_else(|| ElaborationError::InvalidModule("assignment: missing lvalue".into()))?;
    let rhs_node = child_rule(node, "expression")
        .ok_or_else(|| ElaborationError::InvalidModule("assignment: missing rhs".into()))?;
    Ok(ContAssign {
        target: elaborate_lvalue(lv)?,
        rhs: elaborate_expr(rhs_node)?,
        provenance: None,
    })
}

fn elaborate_lvalue(node: &GrammarASTNode) -> Result<Expr, ElaborationError> {
    if let Some(concat) = child_rule(node, "concatenation") {
        return elaborate_expr(concat);
    }
    let name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if is_name_token(t) => Some(t.value.as_str()),
        _ => None,
    }).ok_or_else(|| ElaborationError::InvalidModule("lvalue: no name".into()))?;

    if let Some(rs) = child_rule(node, "range_select") {
        let exprs = child_rules(rs, "expression");
        if exprs.len() >= 2 {
            let msb = first_number(exprs[0]).unwrap_or(0);
            let lsb = first_number(exprs[1]).unwrap_or(0);
            return Ok(Expr::Slice {
                base: Box::new(Expr::port_ref(name)),
                msb, lsb, provenance: None,
            });
        } else if exprs.len() == 1 {
            let idx = first_number(exprs[0]).unwrap_or(0);
            return Ok(Expr::Slice {
                base: Box::new(Expr::port_ref(name)),
                msb: idx, lsb: idx, provenance: None,
            });
        }
    }
    Ok(Expr::port_ref(name))
}
