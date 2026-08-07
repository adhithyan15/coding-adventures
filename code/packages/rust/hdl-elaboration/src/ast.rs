//! Lightweight helpers for navigating `GrammarASTNode` trees.
//!
//! The parser crate's `find_nodes()` returns clones; these helpers work with
//! references so we avoid unnecessary allocations during elaboration.

use lexer::token::TokenType;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ---------------------------------------------------------------------------
// Direct-child queries
// ---------------------------------------------------------------------------

/// Return the first direct child that is a rule node named `rule`.
pub(crate) fn child_rule<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == rule => Some(n),
        _ => None,
    })
}

/// Return all direct children that are rule nodes named `rule`.
pub(crate) fn child_rules<'a>(node: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == rule => Some(n),
        _ => None,
    }).collect()
}

/// Return all direct children that are rule nodes (any name).
pub(crate) fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    }).collect()
}

#[allow(dead_code)]
/// Return true if any direct child token has value `val`.
pub(crate) fn has_direct_token(node: &GrammarASTNode, val: &str) -> bool {
    node.children.iter().any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == val))
}

/// Collect all direct-child token values.
pub(crate) fn direct_token_values(node: &GrammarASTNode) -> Vec<&str> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
        _ => None,
    }).collect()
}

// ---------------------------------------------------------------------------
// Deep search
// ---------------------------------------------------------------------------

#[allow(dead_code)]
/// Depth-first search for the first rule node named `rule` anywhere in the tree.
pub(crate) fn find_first<'a>(node: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    if node.rule_name == rule { return Some(node); }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            if let Some(found) = find_first(n, rule) { return Some(found); }
        }
    }
    None
}

/// Find all rule nodes named `rule` at any depth (depth-first, including root).
pub(crate) fn find_all<'a>(node: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    let mut out = Vec::new();
    collect_all(node, rule, &mut out);
    out
}

fn collect_all<'a>(node: &'a GrammarASTNode, rule: &str, out: &mut Vec<&'a GrammarASTNode>) {
    if node.rule_name == rule { out.push(node); }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child { collect_all(n, rule, out); }
    }
}

// ---------------------------------------------------------------------------
// Token extraction
// ---------------------------------------------------------------------------

/// Walk a node, unwrapping single-child rule nodes, until we reach a leaf or a
/// multi-child node. Returns the token value if the final leaf is a token.
pub(crate) fn unwrap_to_token_value(node: &GrammarASTNode) -> Option<&str> {
    if let Some(tok) = node.token() { return Some(tok.value.as_str()); }
    if node.children.len() == 1 {
        match &node.children[0] {
            ASTNodeOrToken::Node(n) => return unwrap_to_token_value(n),
            ASTNodeOrToken::Token(t) => return Some(t.value.as_str()),
        }
    }
    None
}

#[allow(dead_code)]
/// Find the first NAME-typed token value under `node` (depth-first).
pub(crate) fn first_name(node: &GrammarASTNode) -> Option<&str> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if is_name_token(t) => return Some(t.value.as_str()),
            ASTNodeOrToken::Node(n) => {
                if let Some(v) = first_name(n) { return Some(v); }
            }
            _ => {}
        }
    }
    None
}

/// Return true if token is a NAME identifier.
///
/// The Verilog lexer produces `type_: TokenType::Name, type_name: None`
/// for identifiers. Grammar-driven custom tokens carry an explicit type_name.
pub(crate) fn is_name_token(t: &lexer::token::Token) -> bool {
    match t.type_name.as_deref() {
        Some("NAME") => true,
        None => t.type_ == TokenType::Name,
        _ => false,
    }
}

/// Find the first NUMBER or SIZED_NUMBER token value under `node` (depth-first).
pub(crate) fn first_number(node: &GrammarASTNode) -> Option<u32> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if is_number_token(t) => {
                return parse_verilog_number(&t.value);
            }
            ASTNodeOrToken::Node(n) => {
                if let Some(v) = first_number(n) { return Some(v); }
            }
            _ => {}
        }
    }
    None
}

fn is_number_token(t: &lexer::token::Token) -> bool {
    match t.type_name.as_deref() {
        Some("NUMBER") | Some("SIZED_NUMBER") => true,
        None => t.type_ == TokenType::Number,
        _ => false,
    }
}

/// Parse a Verilog integer literal into u32.
///
/// Handles plain decimals (`42`) and sized literals (`8'h1f`, `4'b1010`,
/// `8'd255`). Returns `None` for un-parseable input.
pub(crate) fn parse_verilog_number(s: &str) -> Option<u32> {
    if let Some(apos) = s.find('\'') {
        let base_char = s.as_bytes().get(apos + 1).copied().unwrap_or(b'd') as char;
        if apos + 2 > s.len() { return None; }
        let digits = &s[apos + 2..];
        let v = match base_char.to_ascii_lowercase() {
            'b' => u32::from_str_radix(digits, 2).ok()?,
            'o' => u32::from_str_radix(digits, 8).ok()?,
            'd' => digits.parse::<u32>().ok()?,
            'h' => u32::from_str_radix(digits, 16).ok()?,
            _ => return None,
        };
        Some(v)
    } else {
        s.parse::<u32>().ok()
    }
}
