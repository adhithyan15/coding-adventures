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
}

impl Checker {
    fn check_program(&mut self, root: &GrammarASTNode) {
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

    fn check_function(&mut self, fn_decl: &GrammarASTNode) {
        let mut env = HashMap::new();
        if let Some(block) = child_nodes(fn_decl)
            .into_iter()
            .find(|node| node.rule_name == "block")
        {
            self.check_block(block, &mut env);
        }
    }

    fn check_block(&mut self, block: &GrammarASTNode, env: &mut HashMap<String, NibType>) {
        for stmt in child_nodes(block) {
            self.check_stmt(stmt, env);
        }
    }

    fn check_stmt(&mut self, stmt: &GrammarASTNode, env: &mut HashMap<String, NibType>) {
        if stmt.rule_name == "stmt" {
            if let Some(inner) = child_nodes(stmt).first() {
                self.check_stmt(inner, env);
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
                    if let Some(actual) = self.infer_expr(expr, env) {
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
                        if let Some(actual) = self.infer_expr(expr, env) {
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
            _ => {}
        }
    }

    fn infer_expr(
        &mut self,
        node: &GrammarASTNode,
        env: &HashMap<String, NibType>,
    ) -> Option<NibType> {
        let key = node as *const GrammarASTNode as usize;
        if let Some(existing) = self.types.get(&key) {
            return Some(existing.clone());
        }

        let inferred = match node.rule_name.as_str() {
            "expr" | "or_expr" | "and_expr" | "eq_expr" | "cmp_expr" | "bitwise_expr"
            | "unary_expr" => child_nodes(node)
                .first()
                .and_then(|child| self.infer_expr(child, env)),
            "add_expr" => {
                let operands = child_nodes(node);
                let left = operands
                    .first()
                    .and_then(|child| self.infer_expr(child, env));
                let right = operands
                    .get(1)
                    .and_then(|child| self.infer_expr(child, env));
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
            "primary" => infer_primary(node, env),
            "type" => parse_type(node),
            _ => {
                let children = child_nodes(node);
                if children.len() == 1 {
                    self.infer_expr(children[0], env)
                } else {
                    infer_primary(node, env)
                }
            }
        };

        if let Some(ref ty) = inferred {
            self.types.insert(key, ty.clone());
        }
        inferred
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

fn first_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
            Some(token.value.clone())
        }
        ASTNodeOrToken::Token(_) => None,
        ASTNodeOrToken::Node(inner) => first_name(inner),
    })
}

fn first_token_value(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|child| match child {
        ASTNodeOrToken::Token(token) => Some(token.value.clone()),
        ASTNodeOrToken::Node(inner) => first_token_value(inner),
    })
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

fn infer_primary(node: &GrammarASTNode, env: &HashMap<String, NibType>) -> Option<NibType> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) if token.value == "true" || token.value == "false" => {
                return Some(NibType::Bool);
            }
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "INT_LIT" => {
                let value: i64 = token.value.parse().ok()?;
                return Some(if value <= 15 {
                    NibType::U4
                } else {
                    NibType::U8
                });
            }
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "HEX_LIT" => {
                let value = i64::from_str_radix(token.value.trim_start_matches("0x"), 16).ok()?;
                return Some(if value <= 15 {
                    NibType::U4
                } else {
                    NibType::U8
                });
            }
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => {
                if let Some(found) = env.get(&token.value) {
                    return Some(found.clone());
                }
            }
            ASTNodeOrToken::Node(inner) => {
                if let Some(found) = infer_primary(inner, env) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
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
}
