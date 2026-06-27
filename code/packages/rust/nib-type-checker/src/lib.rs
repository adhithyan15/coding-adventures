use std::collections::HashMap;

use coding_adventures_nib_parser::parse_nib;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use type_checker_protocol::{TypeCheckResult, TypeErrorDiagnostic};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NibType {
    U4,
    U8,
    Bcd,
    Bool,
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedAst {
    pub root: GrammarASTNode,
    pub types: HashMap<usize, NibType>,
}

pub fn check_source(source: &str) -> TypeCheckResult<TypedAst> {
    match parse_nib(source) {
        Ok(ast) => check(ast),
        Err(err) => TypeCheckResult {
            typed_ast: TypedAst {
                root: GrammarASTNode {
                    rule_name: "program".to_string(),
                    children: vec![],
                    start_line: None,
                    start_column: None,
                    end_line: None,
                    end_column: None,
                },
                types: HashMap::new(),
            },
            errors: vec![TypeErrorDiagnostic {
                message: err.to_string(),
                line: err.token.line,
                column: err.token.column,
            }],
            ok: false,
        },
    }
}

pub fn check(root: GrammarASTNode) -> TypeCheckResult<TypedAst> {
    let mut checker = Checker {
        errors: Vec::new(),
        types: HashMap::new(),
        consts: HashMap::new(),
        statics: HashMap::new(),
    };
    checker.check_program(&root);
    let ok = checker.errors.is_empty();
    TypeCheckResult {
        typed_ast: TypedAst {
            root,
            types: checker.types,
        },
        errors: checker.errors,
        ok,
    }
}

struct Checker {
    errors: Vec<TypeErrorDiagnostic>,
    types: HashMap<usize, NibType>,
    consts: HashMap<String, NibType>,
    statics: HashMap<String, NibType>,
}

impl Checker {
    fn check_program(&mut self, root: &GrammarASTNode) {
        self.collect_const_declarations(root);
        self.collect_static_declarations(root);
        self.check_const_initializers(root);
        self.check_static_initializers(root);
        for decl in child_nodes(root) {
            if decl.rule_name == "fn_decl" {
                self.check_function(decl);
            } else if decl.rule_name == "top_decl" {
                if let Some(inner) = child_nodes(decl).first() {
                    if inner.rule_name == "fn_decl" {
                        self.check_function(inner);
                    }
                }
            }
        }
    }

    fn collect_const_declarations(&mut self, root: &GrammarASTNode) {
        for decl in child_nodes(root) {
            let Some(const_decl) = const_decl_node(decl) else {
                continue;
            };
            let name = first_name(const_decl).unwrap_or_else(|| "<unknown>".to_string());
            let declared = child_nodes(const_decl)
                .into_iter()
                .find(|node| node.rule_name == "type")
                .and_then(parse_type)
                .unwrap_or(NibType::U4);
            self.consts.insert(name, declared);
        }
    }

    fn collect_static_declarations(&mut self, root: &GrammarASTNode) {
        for decl in child_nodes(root) {
            let Some(static_decl) = static_decl_node(decl) else {
                continue;
            };
            let name = first_name(static_decl).unwrap_or_else(|| "<unknown>".to_string());
            let declared = child_nodes(static_decl)
                .into_iter()
                .find(|node| node.rule_name == "type")
                .and_then(parse_type)
                .unwrap_or(NibType::U4);
            self.statics.insert(name, declared);
        }
    }

    fn check_const_initializers(&mut self, root: &GrammarASTNode) {
        for decl in child_nodes(root) {
            let Some(const_decl) = const_decl_node(decl) else {
                continue;
            };
            let name = first_name(const_decl).unwrap_or_else(|| "<unknown>".to_string());
            let declared = child_nodes(const_decl)
                .into_iter()
                .find(|node| node.rule_name == "type")
                .and_then(parse_type)
                .unwrap_or(NibType::U4);
            let expr = child_nodes(const_decl)
                .into_iter()
                .find(|node| node.rule_name == "expr");
            if let Some(expr) = expr {
                let env = self.consts.clone();
                if let Some(actual) = self.infer_expr(expr, &env, Some(&declared)) {
                    if actual != declared {
                        self.error(
                            format!("const `{name}` expects {:?}, got {:?}", declared, actual),
                            expr,
                        );
                    }
                }
            }
        }
    }

    fn check_static_initializers(&mut self, root: &GrammarASTNode) {
        for decl in child_nodes(root) {
            let Some(static_decl) = static_decl_node(decl) else {
                continue;
            };
            let name = first_name(static_decl).unwrap_or_else(|| "<unknown>".to_string());
            let declared = child_nodes(static_decl)
                .into_iter()
                .find(|node| node.rule_name == "type")
                .and_then(parse_type)
                .unwrap_or(NibType::U4);
            let expr = child_nodes(static_decl)
                .into_iter()
                .find(|node| node.rule_name == "expr");
            if let Some(expr) = expr {
                let env = self.consts.clone();
                if let Some(actual) = self.infer_expr(expr, &env, Some(&declared)) {
                    if actual != declared {
                        self.error(
                            format!("static `{name}` expects {:?}, got {:?}", declared, actual),
                            expr,
                        );
                    }
                }
            }
        }
    }

    fn check_function(&mut self, fn_decl: &GrammarASTNode) {
        let mut env = self.consts.clone();
        env.extend(self.statics.clone());
        // Seed the env with the parameters' declared types so their uses type.
        for (name, ty) in extract_params(fn_decl) {
            env.insert(name, ty);
        }
        // The declared return type is the context for every `return <expr>`.
        let ret_ty = child_nodes(fn_decl)
            .into_iter()
            .find(|node| node.rule_name == "type")
            .and_then(parse_type);
        if let Some(block) = child_nodes(fn_decl)
            .into_iter()
            .find(|node| node.rule_name == "block")
        {
            self.check_block(block, &mut env, ret_ty.as_ref());
        }
    }

    fn check_block(
        &mut self,
        block: &GrammarASTNode,
        env: &mut HashMap<String, NibType>,
        ret_ty: Option<&NibType>,
    ) {
        for stmt in child_nodes(block) {
            self.check_stmt(stmt, env, ret_ty);
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &GrammarASTNode,
        env: &mut HashMap<String, NibType>,
        ret_ty: Option<&NibType>,
    ) {
        if stmt.rule_name == "stmt" {
            if let Some(inner) = child_nodes(stmt).first() {
                self.check_stmt(inner, env, ret_ty);
            }
            return;
        }
        match stmt.rule_name.as_str() {
            "let_stmt" => {
                let name = first_name(stmt).unwrap_or_else(|| "<unknown>".to_string());
                let declared = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "type")
                    .and_then(parse_type)
                    .unwrap_or(NibType::U4);
                let expr = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "expr");
                if let Some(expr) = expr {
                    // The declared type is the context for the RHS (E2).
                    if let Some(actual) = self.infer_expr(expr, env, Some(&declared)) {
                        if actual != declared {
                            self.error(
                                format!("let `{name}` expects {:?}, got {:?}", declared, actual),
                                expr,
                            );
                        }
                    }
                }
                env.insert(name, declared);
            }
            "assign_stmt" => {
                let name = first_name(stmt).unwrap_or_else(|| "<unknown>".to_string());
                let expr = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "expr");
                if let Some(expected) = env.get(&name).cloned() {
                    if let Some(expr) = expr {
                        if let Some(actual) = self.infer_expr(expr, env, Some(&expected)) {
                            if actual != expected {
                                self.error(
                                    format!(
                                        "assignment to `{name}` expects {:?}, got {:?}",
                                        expected, actual
                                    ),
                                    expr,
                                );
                            }
                        }
                    }
                } else {
                    self.error(format!("unknown variable `{name}`"), stmt);
                }
            }
            "return_stmt" => {
                // The function's declared return type is the context, so
                // `return 6 * 7` in a `-> u8` fn types the `6 * 7` as u8 (E2).
                if let Some(expr) = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "expr")
                {
                    self.infer_expr(expr, env, ret_ty);
                }
            }
            "for_stmt" => {
                // `for NAME : type in lo .. hi block` — the loop variable's
                // declared type is the context for both range bounds and is in
                // scope for the body.
                let var = first_name(stmt).unwrap_or_else(|| "<loopvar>".to_string());
                let var_ty = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "type")
                    .and_then(parse_type)
                    .unwrap_or(NibType::U4);
                for bound in child_nodes(stmt)
                    .into_iter()
                    .filter(|node| node.rule_name == "expr")
                {
                    self.infer_expr(bound, env, Some(&var_ty));
                }
                let mut body_env = env.clone();
                body_env.insert(var, var_ty);
                if let Some(body) = child_nodes(stmt)
                    .into_iter()
                    .find(|node| node.rule_name == "block")
                {
                    self.check_block(body, &mut body_env, ret_ty);
                }
            }
            "if_stmt" => {
                // Recurse into branch blocks so their statements are typed too.
                for child in child_nodes(stmt) {
                    if child.rule_name == "block" {
                        let mut branch_env = env.clone();
                        self.check_block(child, &mut branch_env, ret_ty);
                    } else if child.rule_name == "expr" {
                        // The condition is a boolean predicate.
                        self.infer_expr(child, env, Some(&NibType::Bool));
                    }
                }
            }
            _ => {}
        }
    }

    /// Infer the type of an expression node, **bidirectionally**: `expected`
    /// is the type the surrounding context wants this expression to have (the
    /// declared type of the `let`/`assign`/`for` binding, or the function's
    /// return type). LANG-FULL E2 needs this: a bare integer literal has no
    /// intrinsic width, so `6 * 7` in `fn … -> u8 { return 6 * 7 }` must be
    /// `u8` (→ wraps at 0xFF, stays 42), **not** `u4` inferred from the
    /// operands' magnitude (which would wrap `42 & 0xF = 10`). The expected
    /// type flows *down* through the arithmetic/bitwise levels to the literals,
    /// which adopt it (when it is a numeric width that fits); only a truly
    /// unconstrained literal falls back to the magnitude heuristic.
    fn infer_expr(
        &mut self,
        node: &GrammarASTNode,
        env: &HashMap<String, NibType>,
        expected: Option<&NibType>,
    ) -> Option<NibType> {
        let key = node as *const GrammarASTNode as usize;
        if let Some(existing) = self.types.get(&key) {
            return Some(existing.clone());
        }

        let inferred = match node.rule_name.as_str() {
            "expr" | "or_expr" | "and_expr" | "eq_expr" | "cmp_expr" | "bitwise_expr" => {
                child_nodes(node)
                    .first()
                    .and_then(|child| self.infer_expr(child, env, expected))
            }
            "unary_expr" => self.infer_unary_expr(node, env, expected),
            "add_expr" | "mul_expr" => {
                let operands = child_nodes(node);
                // The expected width flows to BOTH operands (an arithmetic op
                // preserves width), so `200 + 100 : u8` types each literal u8.
                let left = operands
                    .first()
                    .and_then(|child| self.infer_expr(child, env, expected));
                let right = operands
                    .get(1)
                    .and_then(|child| self.infer_expr(child, env, expected));
                match (left, right) {
                    (Some(a), Some(b)) if a == b && is_numeric(&a) => Some(a),
                    (Some(a), None) => Some(a),
                    (Some(a), Some(b)) => {
                        self.error(
                            format!("binary expression type mismatch: {:?} vs {:?}", a, b),
                            node,
                        );
                        None
                    }
                    _ => None,
                }
            }
            "primary" => infer_primary(node, env, expected),
            "type" => parse_type(node),
            _ => {
                let children = child_nodes(node);
                if children.len() == 1 {
                    self.infer_expr(children[0], env, expected)
                } else {
                    infer_primary(node, env, expected)
                }
            }
        };

        if let Some(ref ty) = inferred {
            self.types.insert(key, ty.clone());
        }
        inferred
    }

    fn infer_unary_expr(
        &mut self,
        node: &GrammarASTNode,
        env: &HashMap<String, NibType>,
        expected: Option<&NibType>,
    ) -> Option<NibType> {
        let inner = child_nodes(node)
            .into_iter()
            .find(|child| is_expr_rule(&child.rule_name))?;
        let op = node.children.iter().find_map(|child| match child {
            ASTNodeOrToken::Token(token) => Some(token),
            ASTNodeOrToken::Node(_) => None,
        });

        match op.map(|token| (token.value.as_str(), token.effective_type_name())) {
            Some(("!", _)) | Some((_, "BANG")) => {
                self.infer_expr(inner, env, None);
                Some(NibType::Bool)
            }
            Some(("~", _)) | Some((_, "TILDE")) => self.infer_expr(inner, env, expected),
            _ => self.infer_expr(inner, env, expected),
        }
    }

    fn error(&mut self, message: impl Into<String>, node: &GrammarASTNode) {
        self.errors.push(TypeErrorDiagnostic {
            message: message.into(),
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        });
    }
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(inner) => Some(inner),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn is_expr_rule(name: &str) -> bool {
    matches!(
        name,
        "expr"
            | "or_expr"
            | "and_expr"
            | "eq_expr"
            | "cmp_expr"
            | "add_expr"
            | "mul_expr"
            | "bitwise_expr"
            | "unary_expr"
            | "primary"
            | "call_expr"
    )
}

fn first_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        ASTNodeOrToken::Token(_) => None,
        ASTNodeOrToken::Node(inner) => first_name(inner),
    })
}

fn const_decl_node(decl: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if decl.rule_name == "const_decl" {
        Some(decl)
    } else if decl.rule_name == "top_decl" {
        child_nodes(decl)
            .into_iter()
            .find(|node| node.rule_name == "const_decl")
    } else {
        None
    }
}

fn static_decl_node(decl: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if decl.rule_name == "static_decl" {
        Some(decl)
    } else if decl.rule_name == "top_decl" {
        child_nodes(decl)
            .into_iter()
            .find(|node| node.rule_name == "static_decl")
    } else {
        None
    }
}

fn first_token_value(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) => Some(token.value.clone()),
        ASTNodeOrToken::Node(inner) => first_token_value(inner),
    })
}

/// Extract `(param_name, NibType)` pairs from a `fn_decl`'s `param_list`
/// (`param = NAME COLON type`). Used to seed the function env so a parameter's
/// uses (and any `return param`/arithmetic over it) type at its declared width.
fn extract_params(fn_decl: &GrammarASTNode) -> Vec<(String, NibType)> {
    let mut out = Vec::new();
    for child in child_nodes(fn_decl) {
        if child.rule_name == "param_list" {
            for param in child_nodes(child) {
                if param.rule_name == "param" {
                    if let (Some(name), Some(ty)) = (
                        first_name(param),
                        child_nodes(param)
                            .into_iter()
                            .find(|n| n.rule_name == "type")
                            .and_then(parse_type),
                    ) {
                        out.push((name, ty));
                    }
                }
            }
        }
    }
    out
}

fn parse_type(node: &GrammarASTNode) -> Option<NibType> {
    match first_token_value(node)?.as_str() {
        "u4" => Some(NibType::U4),
        "u8" => Some(NibType::U8),
        "bcd" => Some(NibType::Bcd),
        "bool" => Some(NibType::Bool),
        _ => None,
    }
}

fn infer_primary(
    node: &GrammarASTNode,
    env: &HashMap<String, NibType>,
    expected: Option<&NibType>,
) -> Option<NibType> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) if token.value == "true" || token.value == "false" => {
                return Some(NibType::Bool);
            }
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "INT_LIT" => {
                let value: i64 = token.value.parse().ok()?;
                return Some(literal_width(value, expected));
            }
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "HEX_LIT" => {
                let value = i64::from_str_radix(token.value.trim_start_matches("0x"), 16).ok()?;
                return Some(literal_width(value, expected));
            }
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
                if let Some(found) = env.get(&token.value) {
                    return Some(found.clone());
                }
            }
            ASTNodeOrToken::Node(inner) => {
                if let Some(found) = infer_primary(inner, env, expected) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// The type of an integer literal under bidirectional inference. When the
/// context supplies an `expected` numeric width that the value fits, the
/// literal adopts it (so `6` in a `u8` context is `u8`, not `u4`). Otherwise
/// it falls back to the magnitude heuristic (`≤15 → u4`, else `u8`) — the
/// behaviour of an unconstrained literal before E2.
fn literal_width(value: i64, expected: Option<&NibType>) -> NibType {
    match expected {
        Some(NibType::U4) if (0..=0xF).contains(&value) => NibType::U4,
        Some(NibType::U8) if (0..=0xFF).contains(&value) => NibType::U8,
        Some(NibType::Bcd) if (0..=9).contains(&value) => NibType::Bcd,
        // No usable context (or the value doesn't fit it — a separate width
        // check elsewhere reports that): fall back to the magnitude heuristic.
        _ => {
            if value <= 15 {
                NibType::U4
            } else {
                NibType::U8
            }
        }
    }
}

fn is_numeric(ty: &NibType) -> bool {
    matches!(ty, NibType::U4 | NibType::U8 | NibType::Bcd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_source_accepts_simple_program() {
        let result = check_source("fn main() { let x: u4 = 5; }");
        assert!(result.ok, "expected success, got {:?}", result.errors);
    }

    #[test]
    fn check_source_rejects_bad_assignment() {
        let result = check_source("fn main() { let x: bool = 1 +% 2; }");
        assert!(!result.ok);
    }

    #[test]
    fn check_source_accepts_static_assignment_across_functions() {
        let result = check_source(
            "static counter: u8 = 40; \
             fn bump(step: u8) -> u8 { counter = counter + step; return counter; } \
             fn main() -> u8 { let a: u8 = bump(1); let b: u8 = bump(1); return counter; }",
        );
        assert!(result.ok, "expected success, got {:?}", result.errors);
    }

    #[test]
    fn check_source_rejects_static_initializer_wrong_width() {
        let result = check_source("static small: u4 = 16; fn main() {}");
        assert!(!result.ok);
        assert!(
            result
                .errors
                .iter()
                .any(|err| err.message.contains("static `small` expects")),
            "expected static width error, got {:?}",
            result.errors
        );
    }

    #[test]
    fn check_source_accepts_const_expression_initializer() {
        let result = check_source("const N: u8 = 6 * 7; fn main() -> u8 { return N; }");
        assert!(result.ok, "expected success, got {:?}", result.errors);
    }

    #[test]
    fn check_source_rejects_const_initializer_wrong_width() {
        let result = check_source("const small: u4 = 16; fn main() {}");
        assert!(!result.ok);
        assert!(
            result
                .errors
                .iter()
                .any(|err| err.message.contains("const `small` expects")),
            "expected const width error, got {:?}",
            result.errors
        );
    }

    #[test]
    fn check_source_accepts_static_expression_using_const() {
        let result =
            check_source("const BASE: u8 = 40 + 1; static counter: u8 = BASE + 1; fn main() {}");
        assert!(result.ok, "expected success, got {:?}", result.errors);
    }

    #[test]
    fn check_source_accepts_logical_not_condition() {
        let result = check_source("fn main() -> u8 { if !(1 == 2) { return 42; } return 0; }");
        assert!(result.ok, "expected success, got {:?}", result.errors);
    }

    #[test]
    fn check_source_accepts_logical_not_bool_binding() {
        let result = check_source("fn main() { let b: bool = !false; }");
        assert!(result.ok, "expected success, got {:?}", result.errors);
    }
}
