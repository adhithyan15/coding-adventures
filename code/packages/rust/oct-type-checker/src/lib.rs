//! # `oct-type-checker` — Oct type checker (OCT02 phase 2).
//!
//! Walks the AST produced by `coding-adventures-oct-parser` and
//! verifies Oct's language-level type invariants:
//!
//! - All names declared before use.
//! - Arithmetic / bitwise ops require `u8`-compatible operands.
//! - Logical ops require `bool` operands.
//! - `if` / `while` conditions must be `bool`.
//! - Assignment / `return` / call args respect the declared types.
//! - `bool → u8` is the only implicit coercion; `u8 → bool` is rejected
//!   (the classic `if (x)` foot-gun is closed).
//! - `main` exists with no params and void return type.
//! - Integer literals fit in `u8` (0..=255).
//!
//! ## Out of scope
//!
//! - Hardware-specific limits (max 4 locals, max 7 call depth, port
//!   ranges 0..=7 / 0..=23, ≤ 16 KB program size) — those belong to the
//!   8008 simulator backend, not the language layer.
//! - 8008 intrinsic argument validation — V1 lets intrinsics through
//!   type-checking; the iir-compiler later rejects them with a clean
//!   `Unsupported8008Intrinsic` error.  (The Python checker validates
//!   intrinsic arg types; the Rust port intentionally simplifies because
//!   the AOT chain never compiles them anyway.)
//!
//! ## API
//!
//! ```no_run
//! use oct_type_checker::check_source;
//!
//! let result = check_source("fn main() { let x: u8 = 42; }");
//! assert!(result.ok);
//! assert!(result.errors.is_empty());
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

use coding_adventures_oct_parser::parse_oct;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ===========================================================================
// Types
// ===========================================================================

/// An Oct value type, represented as a string so downstream consumers
/// (oct-iir-compiler) can read it without depending on this crate's enum.
///
/// `"u8"` — unsigned 8-bit integer (0..=255).
/// `"bool"` — boolean stored as 0/1 in a u8 register.
/// `"void"` — internal-only sentinel for functions with no return type.
pub type OctType = String;

const VALID_TYPES: &[&str] = &["u8", "bool"];

fn is_u8_compatible(ty: Option<&str>) -> bool {
    matches!(ty, Some("u8") | Some("bool"))
}

fn assignable(src: Option<&str>, dst: Option<&str>) -> bool {
    // None on either side is an "already-reported" sentinel; tolerate it.
    let (Some(s), Some(d)) = (src, dst) else { return true; };
    s == d || (s == "bool" && d == "u8")
}

/// A single type-check diagnostic.
#[derive(Debug, Clone)]
pub struct TypeError {
    /// Human-readable message.
    pub message: String,
    /// 1-based source line.
    pub line: usize,
    /// 1-based source column.
    pub column: usize,
}

/// Result of a type-check pass.
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// `true` iff `errors` is empty.
    pub ok: bool,
    /// All diagnostics produced during checking.  Order matches walk order
    /// (top of file → bottom).
    pub errors: Vec<TypeError>,
}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Lex + parse + type-check an Oct source string.  Convenience wrapper
/// around [`check_ast`].
pub fn check_source(source: &str) -> TypeCheckResult {
    match parse_oct(source) {
        Ok(ast) => check_ast(&ast),
        Err(e) => TypeCheckResult {
            ok: false,
            errors: vec![TypeError {
                message: format!("parse error: {e}"),
                line: 1,
                column: 1,
            }],
        },
    }
}

/// Type-check a pre-parsed Oct AST.
pub fn check_ast(ast: &GrammarASTNode) -> TypeCheckResult {
    let mut tc = TypeChecker::default();
    tc.pass1_collect(ast);
    tc.verify_main();
    tc.pass2_check(ast);
    TypeCheckResult {
        ok: tc.errors.is_empty(),
        errors: tc.errors,
    }
}

// ===========================================================================
// Checker state
// ===========================================================================

#[derive(Debug, Clone)]
struct FnInfo {
    params: Vec<(String, OctType)>,
    return_type: Option<OctType>,
}

#[derive(Default)]
struct TypeChecker {
    /// Module-level static declarations: name → type.
    statics: HashMap<String, OctType>,
    /// Function signatures from pass 1.
    functions: HashMap<String, FnInfo>,
    /// Accumulating diagnostics.
    errors: Vec<TypeError>,
}

impl TypeChecker {
    fn err(&mut self, message: impl Into<String>, line: usize, column: usize) {
        self.errors.push(TypeError { message: message.into(), line, column });
    }

    fn err_at(&mut self, message: impl Into<String>, node: &GrammarASTNode) {
        let (line, column) = node_loc(node);
        self.err(message, line, column);
    }

    fn resolve_name(&self, name: &str, local: &HashMap<String, OctType>) -> Option<OctType> {
        local.get(name).or_else(|| self.statics.get(name)).cloned()
    }

    // ── Pass 1 — signature collection ───────────────────────────────────────

    fn pass1_collect(&mut self, program: &GrammarASTNode) {
        for child in child_nodes(program) {
            if child.rule_name != "top_decl" { continue; }
            for inner in child_nodes(child) {
                match inner.rule_name.as_str() {
                    "static_decl" => self.collect_static(inner),
                    "fn_decl"     => self.collect_fn(inner),
                    _ => {}
                }
            }
        }
    }

    fn collect_static(&mut self, node: &GrammarASTNode) {
        let Some(name) = first_name_token(node) else {
            self.err_at("internal: static_decl has no NAME token", node);
            return;
        };
        let type_node = child_nodes(node).into_iter()
            .find(|n| n.rule_name == "type");
        let Some(type_node) = type_node else {
            self.err_at(format!("static '{name}': missing type annotation"), node);
            return;
        };
        match resolve_type(type_node) {
            // NOTE: `map_entry` would suggest the `entry` API, but the "already
            // declared" branch must call `self.err_at(..)` which needs `&mut self`,
            // conflicting with a live `self.statics` entry borrow. The contains_key +
            // insert form keeps the borrows disjoint, so the lint is allowed here.
            #[allow(clippy::map_entry)]
            Some(t) => {
                if self.statics.contains_key(&name) {
                    self.err_at(format!("static '{name}' is already declared"), node);
                } else {
                    self.statics.insert(name, t);
                }
            }
            None => {
                let raw = first_name_token(type_node).unwrap_or_default();
                self.err_at(format!("unknown type '{raw}' in static '{name}'"), type_node);
            }
        }
    }

    fn collect_fn(&mut self, node: &GrammarASTNode) {
        let Some(name) = first_name_token(node) else {
            self.err_at("internal: fn_decl has no NAME token", node);
            return;
        };
        let params = extract_params(node);
        let return_type = extract_return_type(node);
        if self.functions.contains_key(&name) {
            self.err_at(format!("function '{name}' is already defined"), node);
            return;
        }
        self.functions.insert(name, FnInfo { params, return_type });
    }

    fn verify_main(&mut self) {
        let Some(main) = self.functions.get("main").cloned() else {
            self.err("program must define a 'main' function", 1, 1);
            return;
        };
        if !main.params.is_empty() {
            self.err("'main' must take no parameters", 1, 1);
        }
        if main.return_type.is_some() {
            self.err("'main' must have no return type (void)", 1, 1);
        }
    }

    // ── Pass 2 — body type-checking ─────────────────────────────────────────

    fn pass2_check(&mut self, program: &GrammarASTNode) {
        for top in child_nodes(program) {
            if top.rule_name != "top_decl" { continue; }
            for inner in child_nodes(top) {
                if inner.rule_name == "fn_decl" {
                    self.check_fn_body(inner);
                }
            }
        }
    }

    fn check_fn_body(&mut self, fn_decl: &GrammarASTNode) {
        let Some(fn_name) = first_name_token(fn_decl) else { return; };
        let Some(info) = self.functions.get(&fn_name).cloned() else { return; };
        let mut local: HashMap<String, OctType> = info.params.iter().cloned().collect();
        if let Some(block) = child_nodes(fn_decl).into_iter().find(|n| n.rule_name == "block") {
            self.check_block(block, &mut local, info.return_type.as_deref());
        }
    }

    fn check_block(
        &mut self,
        block: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
        return_type: Option<&str>,
    ) {
        for stmt in child_nodes(block) {
            if stmt.rule_name == "stmt" {
                self.check_stmt(stmt, local, return_type);
            }
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
        return_type: Option<&str>,
    ) {
        for inner in child_nodes(stmt) {
            match inner.rule_name.as_str() {
                "let_stmt"     => self.check_let(inner, local),
                "static_decl"  => { /* already collected; no-op */ }
                "assign_stmt"  => self.check_assign(inner, local),
                "return_stmt"  => self.check_return(inner, local, return_type),
                "if_stmt"      => self.check_if(inner, local, return_type),
                "while_stmt"   => self.check_while(inner, local, return_type),
                "loop_stmt"    => self.check_loop(inner, local, return_type),
                "break_stmt"   => { /* syntactic only */ }
                "expr_stmt"    => { let _ = self.check_expr_stmt(inner, local); }
                _ => {}
            }
        }
    }

    fn check_let(&mut self, node: &GrammarASTNode, local: &mut HashMap<String, OctType>) {
        let Some(name) = first_name_token(node) else {
            self.err_at("internal: let_stmt has no NAME token", node);
            return;
        };
        let type_node = child_nodes(node).into_iter()
            .find(|n| n.rule_name == "type");
        let Some(type_node) = type_node else {
            self.err_at(format!("let '{name}': missing type annotation"), node);
            return;
        };
        let Some(declared) = resolve_type(type_node) else {
            let raw = first_name_token(type_node).unwrap_or_default();
            self.err_at(format!("unknown type '{raw}' in let '{name}'"), type_node);
            return;
        };
        // First expr child (skipping the `type` node) is the initialiser.
        let expr_node = child_nodes(node).into_iter()
            .find(|n| n.rule_name != "type");
        if let Some(expr_node) = expr_node {
            if let Some(actual) = self.check_expr(expr_node, local) {
                if !assignable(Some(&actual), Some(&declared)) {
                    self.err_at(format!(
                        "cannot assign '{}' to '{}' variable '{}'",
                        actual, declared, name,
                    ), expr_node);
                }
            }
        }
        local.insert(name, declared);
    }

    fn check_assign(&mut self, node: &GrammarASTNode, local: &mut HashMap<String, OctType>) {
        let Some(name) = first_name_token(node) else {
            self.err_at("internal: assign_stmt has no NAME token", node);
            return;
        };
        let Some(declared) = self.resolve_name(&name, local) else {
            self.err_at(format!("assignment to undeclared variable '{name}'"), node);
            return;
        };
        if let Some(expr_node) = child_nodes(node).into_iter().next() {
            if let Some(actual) = self.check_expr(expr_node, local) {
                if !assignable(Some(&actual), Some(&declared)) {
                    self.err_at(format!(
                        "cannot assign '{}' to '{}' variable '{}'",
                        actual, declared, name,
                    ), expr_node);
                }
            }
        }
    }

    fn check_return(
        &mut self,
        node: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
        return_type: Option<&str>,
    ) {
        let expr_node = child_nodes(node).into_iter().next();
        match (expr_node, return_type) {
            (None, Some(rt)) => self.err_at(format!(
                "'return' with no value in function returning '{rt}'"), node),
            (Some(e), None) => {
                let _ = self.check_expr(e, local);
                self.err_at("void function must not return a value", e);
            }
            (Some(e), Some(rt)) => {
                if let Some(actual) = self.check_expr(e, local) {
                    if !assignable(Some(&actual), Some(rt)) {
                        self.err_at(format!(
                            "'return' type mismatch: expected '{}', got '{}'",
                            rt, actual,
                        ), e);
                    }
                }
            }
            (None, None) => {}
        }
    }

    fn check_if(
        &mut self,
        node: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
        return_type: Option<&str>,
    ) {
        let mut cond_node: Option<&GrammarASTNode> = None;
        let mut blocks: Vec<&GrammarASTNode> = Vec::new();
        for c in child_nodes(node) {
            if c.rule_name == "block" { blocks.push(c); }
            else if cond_node.is_none() { cond_node = Some(c); }
        }
        if let Some(cond) = cond_node {
            if let Some(ty) = self.check_expr(cond, local) {
                if ty != "bool" {
                    self.err_at(format!(
                        "'if' condition must be 'bool', got '{}' — use an explicit \
                         comparison (e.g. x != 0)",
                        ty,
                    ), cond);
                }
            }
        }
        for blk in blocks {
            let mut scope = local.clone();
            self.check_block(blk, &mut scope, return_type);
        }
    }

    fn check_while(
        &mut self,
        node: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
        return_type: Option<&str>,
    ) {
        let mut cond_node: Option<&GrammarASTNode> = None;
        let mut block_node: Option<&GrammarASTNode> = None;
        for c in child_nodes(node) {
            if c.rule_name == "block" { block_node = Some(c); }
            else if cond_node.is_none() { cond_node = Some(c); }
        }
        if let Some(cond) = cond_node {
            if let Some(ty) = self.check_expr(cond, local) {
                if ty != "bool" {
                    self.err_at(format!(
                        "'while' condition must be 'bool', got '{}' — use an \
                         explicit comparison (e.g. n != 255)",
                        ty,
                    ), cond);
                }
            }
        }
        if let Some(blk) = block_node {
            let mut scope = local.clone();
            self.check_block(blk, &mut scope, return_type);
        }
    }

    fn check_loop(
        &mut self,
        node: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
        return_type: Option<&str>,
    ) {
        if let Some(blk) = child_nodes(node).into_iter().find(|n| n.rule_name == "block") {
            let mut scope = local.clone();
            self.check_block(blk, &mut scope, return_type);
        }
    }

    fn check_expr_stmt(
        &mut self,
        node: &GrammarASTNode,
        local: &mut HashMap<String, OctType>,
    ) -> Option<OctType> {
        let inner = child_nodes(node).into_iter().next()?;
        self.check_expr(inner, local)
    }

    // ── Expression type inference ───────────────────────────────────────────

    fn check_expr(
        &mut self,
        node: &GrammarASTNode,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        match node.rule_name.as_str() {
            "expr" => {
                // `expr` is a transparent alias for `or_expr`.
                child_nodes(node).into_iter().next()
                    .and_then(|c| self.check_expr(c, local))
            }
            "or_expr" | "and_expr" | "eq_expr"
            | "cmp_expr" | "add_expr" | "bitwise_expr"
                => self.check_binary(node, local),
            "unary_expr" => self.check_unary(node, local),
            "primary"    => self.check_primary(node, local),
            _ => {
                // Unknown wrapper — recurse into first ASTNode child.
                child_nodes(node).into_iter().next()
                    .and_then(|c| self.check_expr(c, local))
            }
        }
    }

    // Explicit loop with an internal break condition reads clearer than while-let (allow 1.97 while_let_loop).
    #[allow(clippy::while_let_loop)]
    fn check_binary(
        &mut self,
        node: &GrammarASTNode,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        // Count non-operator children to detect pass-through (single operand)
        // vs proper binary structure.
        let kids: Vec<&ASTNodeOrToken> = node.children.iter().collect();
        let n_operands = kids.iter().filter(|c| match c {
            ASTNodeOrToken::Node(_) => true,
            ASTNodeOrToken::Token(t) => !is_binary_op_token(&token_kind(t)),
        }).count();
        if n_operands == 1 {
            // Pass-through to single child.
            return self.check_expr(
                node_children_first(node)?,
                local,
            );
        }

        // Walk: left [op right]+.
        let mut iter = kids.into_iter();
        let first = iter.next()?;
        let mut left_type: Option<OctType> = self.check_child(first, local);

        loop {
            let Some(op_child) = iter.next() else { break; };
            let op_name = match op_child {
                ASTNodeOrToken::Token(t) => token_kind(t),
                _ => break,
            };
            let Some(rhs) = iter.next() else { break; };
            let right_type = self.check_child(rhs, local);

            match op_name.as_str() {
                "PLUS" | "MINUS" | "AMP" | "PIPE" | "CARET" => {
                    if !is_u8_compatible(left_type.as_deref()) {
                        self.err_at(format!(
                            "operator '{}' requires 'u8' operand, got '{}'",
                            op_name,
                            left_type.clone().unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    if !is_u8_compatible(right_type.as_deref()) {
                        self.err_at(format!(
                            "operator '{}' requires 'u8' operand, got '{}'",
                            op_name,
                            right_type.clone().unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    left_type = Some("u8".to_string());
                }
                "EQ_EQ" | "NEQ" | "LT" | "GT" | "LEQ" | "GEQ" => {
                    if !is_u8_compatible(left_type.as_deref()) {
                        self.err_at(format!(
                            "comparison '{}' requires 'u8' operand, got '{}'",
                            op_name,
                            left_type.clone().unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    if !is_u8_compatible(right_type.as_deref()) {
                        self.err_at(format!(
                            "comparison '{}' requires 'u8' operand, got '{}'",
                            op_name,
                            right_type.clone().unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    left_type = Some("bool".to_string());
                }
                "LAND" | "LOR" => {
                    if left_type.as_deref() != Some("bool") && left_type.is_some() {
                        self.err_at(format!(
                            "operator '{}' requires 'bool' operand, got '{}' — \
                             use an explicit comparison (e.g. x != 0)",
                            op_name, left_type.clone().unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    if right_type.as_deref() != Some("bool") && right_type.is_some() {
                        self.err_at(format!(
                            "operator '{}' requires 'bool' operand, got '{}' — \
                             use an explicit comparison (e.g. y != 0)",
                            op_name, right_type.clone().unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    left_type = Some("bool".to_string());
                }
                _ => { left_type = None; }
            }
        }
        left_type
    }

    fn check_unary(
        &mut self,
        node: &GrammarASTNode,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        let first = node.children.first()?;
        if let ASTNodeOrToken::Token(t) = first {
            let op = token_kind(t);
            if op == "BANG" || op == "TILDE" {
                let operand = node.children.get(1)?;
                let ty = self.check_child(operand, local);
                if op == "BANG" {
                    if let Some(ty_str) = ty.as_deref() {
                        if ty_str != "bool" {
                            self.err_at(format!(
                                "'!' (logical NOT) requires 'bool' operand, got '{}' — \
                                 use an explicit comparison (e.g. x != 0)",
                                ty_str
                            ), node);
                        }
                    }
                    return Some("bool".to_string());
                } else {
                    if !is_u8_compatible(ty.as_deref()) {
                        self.err_at(format!(
                            "'~' (bitwise NOT) requires 'u8' operand, got '{}'",
                            ty.unwrap_or_else(|| "?".into()),
                        ), node);
                    }
                    return Some("u8".to_string());
                }
            }
        }
        // No unary op — pass through.
        self.check_child(first, local)
    }

    fn check_primary(
        &mut self,
        node: &GrammarASTNode,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => match n.rule_name.as_str() {
                    "intrinsic_call" => return self.check_intrinsic(n, local),
                    "call_expr"      => return self.check_call_expr(n, local),
                    "expr"           => return self.check_expr(n, local),
                    _ => return self.check_expr(n, local),
                },
                ASTNodeOrToken::Token(t) => {
                    let kind = token_kind(t);
                    if kind == "LPAREN" || kind == "RPAREN" { continue; }
                    return self.check_token_primary(t, local);
                }
            }
        }
        None
    }

    fn check_token_primary(
        &mut self,
        tok: &lexer::token::Token,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        let kind = token_kind(tok);
        match kind.as_str() {
            "INT_LIT" | "HEX_LIT" | "BIN_LIT" => {
                if let Some(v) = parse_literal_value(&tok.value, &kind) {
                    if !(0..=255).contains(&v) {
                        self.err(
                            format!("integer literal {:?} is out of u8 range 0–255", tok.value),
                            tok.line, tok.column,
                        );
                    }
                }
                Some("u8".to_string())
            }
            "true" | "false" => Some("bool".to_string()),
            "NAME" => {
                match self.resolve_name(&tok.value, local) {
                    Some(t) => Some(t),
                    None => {
                        self.err(
                            format!("undefined variable '{}'", tok.value),
                            tok.line, tok.column,
                        );
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// Helper to dispatch on a token-or-node child.
    fn check_child(
        &mut self,
        child: &ASTNodeOrToken,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        match child {
            ASTNodeOrToken::Node(n) => self.check_expr(n, local),
            ASTNodeOrToken::Token(t) => self.check_token_primary(t, local),
        }
    }

    fn check_call_expr(
        &mut self,
        node: &GrammarASTNode,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        let fn_name = first_name_token(node)?;
        let info = match self.functions.get(&fn_name).cloned() {
            Some(i) => i,
            None => {
                self.err_at(format!("call to undefined function '{fn_name}'"), node);
                return None;
            }
        };
        let arg_list = child_nodes(node).into_iter().find(|n| n.rule_name == "arg_list");
        let mut args: Vec<&GrammarASTNode> = Vec::new();
        if let Some(al) = arg_list {
            for c in child_nodes(al) {
                args.push(c);
            }
        }
        if args.len() != info.params.len() {
            self.err_at(format!(
                "function '{}' expects {} argument(s), got {}",
                fn_name, info.params.len(), args.len(),
            ), node);
        }
        for (i, arg) in args.iter().enumerate() {
            let arg_type = self.check_expr(arg, local);
            if i < info.params.len() {
                let (_, ptype) = &info.params[i];
                if let Some(arg_ty) = arg_type {
                    if !assignable(Some(&arg_ty), Some(ptype)) {
                        self.err_at(format!(
                            "argument {} to '{}': expected '{}', got '{}'",
                            i + 1, fn_name, ptype, arg_ty,
                        ), arg);
                    }
                }
            }
        }
        info.return_type
    }

    /// Intrinsic calls (`in`, `out`, `adc`, `sbb`, `rlc`, `rrc`, `ral`,
    /// `rar`, `carry`, `parity`).  V1 simplification: best-effort
    /// type-inference; the iir-compiler later rejects every intrinsic
    /// with `Unsupported8008Intrinsic`.  We still walk the arg
    /// expressions so they get name-resolved and produce "undefined
    /// variable" errors when relevant.
    fn check_intrinsic(
        &mut self,
        node: &GrammarASTNode,
        local: &HashMap<String, OctType>,
    ) -> Option<OctType> {
        // Type-check each arg expression so user errors surface even
        // though we'll reject the intrinsic at IR-gen time.
        for c in child_nodes(node) {
            let _ = self.check_expr(c, local);
        }
        // Crude return-type table; matches the Python checker.
        let name = node.children.iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
                _ => None,
            })
            .find(|v| matches!(*v,
                "in" | "out" | "adc" | "sbb" | "rlc"
                | "rrc" | "ral" | "rar" | "carry" | "parity"))
            .unwrap_or("");
        match name {
            "carry" | "parity" => Some("bool".to_string()),
            "out"              => None,  // void
            _                  => Some("u8".to_string()),
        }
    }
}

// ===========================================================================
// AST helpers
// ===========================================================================

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    }).collect()
}

fn node_children_first(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    child_nodes(node).into_iter().next()
}

fn first_name_token(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if token_kind(t) == "NAME" {
                return Some(t.value.clone());
            }
        }
    }
    None
}

fn token_kind(t: &lexer::token::Token) -> String {
    t.effective_type_name().to_string()
}

fn is_binary_op_token(kind: &str) -> bool {
    matches!(kind,
        "LOR" | "LAND" | "EQ_EQ" | "NEQ" | "LT" | "GT" | "LEQ" | "GEQ"
        | "PLUS" | "MINUS" | "AMP" | "PIPE" | "CARET"
    )
}

fn node_loc(node: &GrammarASTNode) -> (usize, usize) {
    // Find the first token reachable from the node.
    fn first_tok(n: &GrammarASTNode) -> Option<(usize, usize)> {
        for c in &n.children {
            match c {
                ASTNodeOrToken::Token(t) => return Some((t.line, t.column)),
                ASTNodeOrToken::Node(child) => {
                    if let Some(p) = first_tok(child) { return Some(p); }
                }
            }
        }
        None
    }
    first_tok(node).unwrap_or((
        node.start_line.unwrap_or(1),
        node.start_column.unwrap_or(1),
    ))
}

fn resolve_type(type_node: &GrammarASTNode) -> Option<OctType> {
    let name = first_name_token(type_node)?;
    if VALID_TYPES.contains(&name.as_str()) {
        Some(name)
    } else {
        None
    }
}

fn extract_params(fn_decl: &GrammarASTNode) -> Vec<(String, OctType)> {
    let mut out = Vec::new();
    let Some(plist) = child_nodes(fn_decl).into_iter().find(|n| n.rule_name == "param_list") else {
        return out;
    };
    for p in child_nodes(plist) {
        if p.rule_name != "param" { continue; }
        let Some(name) = first_name_token(p) else { continue; };
        let ty = child_nodes(p).into_iter().find(|n| n.rule_name == "type")
            .and_then(resolve_type)
            .unwrap_or_else(|| "u8".to_string());
        out.push((name, ty));
    }
    out
}

fn extract_return_type(fn_decl: &GrammarASTNode) -> Option<OctType> {
    // Walk children: when we see ARROW, the next `type` node is the return type.
    let mut saw_arrow = false;
    for c in &fn_decl.children {
        match c {
            ASTNodeOrToken::Token(t) if token_kind(t) == "ARROW" => saw_arrow = true,
            ASTNodeOrToken::Node(n) if saw_arrow && n.rule_name == "type" => {
                return resolve_type(n);
            }
            _ => {}
        }
    }
    None
}

fn parse_literal_value(value: &str, kind: &str) -> Option<i64> {
    match kind {
        "INT_LIT" => value.parse::<i64>().ok(),
        "HEX_LIT" => i64::from_str_radix(value.trim_start_matches("0x"), 16).ok(),
        "BIN_LIT" => i64::from_str_radix(value.trim_start_matches("0b"), 2).ok(),
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_main_passes() {
        let r = check_source("fn main() { let x: u8 = 42; }");
        assert!(r.ok, "expected ok, got errors: {:?}", r.errors);
    }

    #[test]
    fn missing_main_errors() {
        let r = check_source("fn other() { }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("'main'")));
    }

    #[test]
    fn main_with_params_errors() {
        let r = check_source("fn main(x: u8) { }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("no parameters")));
    }

    #[test]
    fn main_with_return_type_errors() {
        let r = check_source("fn main() -> u8 { return 0; }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("no return type")));
    }

    #[test]
    fn u8_to_bool_rejected() {
        let r = check_source("fn main() { let x: u8 = 5; let y: bool = x; }");
        assert!(!r.ok, "u8 → bool must error");
        assert!(r.errors.iter().any(|e| e.message.contains("cannot assign 'u8' to 'bool'")));
    }

    #[test]
    fn bool_to_u8_allowed() {
        let r = check_source("fn main() { let x: u8 = true; }");
        assert!(r.ok, "bool → u8 must be implicitly allowed; got {:?}", r.errors);
    }

    #[test]
    fn if_condition_must_be_bool() {
        let r = check_source("fn main() { let x: u8 = 1; if x { } }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("'if' condition must be 'bool'")));
    }

    #[test]
    fn while_condition_must_be_bool() {
        let r = check_source("fn main() { let x: u8 = 1; while x { } }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("'while' condition must be 'bool'")));
    }

    #[test]
    fn undefined_variable_errors() {
        let r = check_source("fn main() { let x: u8 = nope; }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("undefined variable")));
    }

    #[test]
    fn call_to_undefined_function_errors() {
        let r = check_source("fn main() { let x: u8 = nope(); }");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("undefined function")));
    }

    #[test]
    fn user_fn_call_typechecks() {
        let r = check_source(
            "fn add(a: u8, b: u8) -> u8 { return a + b; } \
             fn main() { let x: u8 = add(1, 2); }",
        );
        assert!(r.ok, "got errors: {:?}", r.errors);
    }

    #[test]
    fn arg_count_mismatch_errors() {
        let r = check_source(
            "fn add(a: u8, b: u8) -> u8 { return a + b; } \
             fn main() { let x: u8 = add(1); }",
        );
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("expects 2 argument")));
    }

    #[test]
    fn arithmetic_returns_u8() {
        // `let y: u8 = x + 1;` should type-check.
        let r = check_source("fn main() { let x: u8 = 5; let y: u8 = x + 1; }");
        assert!(r.ok, "got errors: {:?}", r.errors);
    }

    #[test]
    fn comparison_returns_bool() {
        let r = check_source(
            "fn main() { let x: u8 = 5; let b: bool = x == 0; if b { } }",
        );
        assert!(r.ok, "got errors: {:?}", r.errors);
    }

    #[test]
    fn return_type_mismatch_errors() {
        // `return true` from a `-> u8` function: bool→u8 is implicitly OK
        // (matching the language's coercion rule), so use the inverse for a
        // negative case: a `-> bool` function returning a u8 expression.
        let r = check_source(
            "fn flip() -> bool { let x: u8 = 1; return x; } \
             fn main() { }",
        );
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("'return' type mismatch")));
    }

    #[test]
    fn loop_break_accepted() {
        let r = check_source("fn main() { loop { break; } }");
        assert!(r.ok, "got errors: {:?}", r.errors);
    }

    #[test]
    fn static_decl_typechecks() {
        let r = check_source(
            "static GREETING: u8 = 65; \
             fn main() { let c: u8 = GREETING; }",
        );
        assert!(r.ok, "got errors: {:?}", r.errors);
    }

    #[test]
    fn duplicate_static_errors() {
        let r = check_source(
            "static G: u8 = 1; \
             static G: u8 = 2; \
             fn main() { }",
        );
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("already declared")));
    }

    #[test]
    fn duplicate_function_errors() {
        let r = check_source(
            "fn f() { } \
             fn f() { } \
             fn main() { }",
        );
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.message.contains("already defined")));
    }
}
