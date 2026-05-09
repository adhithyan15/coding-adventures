//! MACSYMA AST to symbolic IR compiler.
//!
//! The parser remains grammar-driven; this crate only maps the generic parser
//! AST into canonical `symbolic-ir` trees.

use std::error::Error;
use std::fmt;

use coding_adventures_macsyma_parser::parse_macsyma;
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{
    apply, flt, int, str_node, sym, IRNode, ACOS, ACOSH, ADD, AND, ASIN, ASINH, ASSIGN, ATAN,
    ATANH, COS, COSH, D, DEFINE, DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, IF, INTEGRATE, LESS,
    LESS_EQUAL, LIST, LOG, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SIN, SINH, SQRT, SUB, TAN, TANH,
};

pub const DISPLAY: &str = "Display";
pub const SUPPRESS: &str = "Suppress";
pub const BLOCK: &str = "Block";
pub const FOR_EACH: &str = "ForEach";
pub const FOR_RANGE: &str = "ForRange";
pub const RETURN: &str = "Return";
pub const WHILE: &str = "While";

#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub wrap_terminators: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    message: String,
}

impl CompileError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CompileError {}

pub fn compile_macsyma(source: &str) -> Result<Vec<IRNode>, CompileError> {
    compile_macsyma_with_options(source, CompileOptions::default())
}

pub fn compile_macsyma_with_options(
    source: &str,
    options: CompileOptions,
) -> Result<Vec<IRNode>, CompileError> {
    let ast = parse_macsyma(source);
    Compiler::new(options).compile_program(&ast)
}

pub struct Compiler {
    options: CompileOptions,
}

impl Compiler {
    pub fn new(options: CompileOptions) -> Self {
        Self { options }
    }

    pub fn compile_program(&self, root: &GrammarASTNode) -> Result<Vec<IRNode>, CompileError> {
        if root.rule_name != "program" {
            return Err(CompileError::new(format!(
                "expected program root, got {}",
                root.rule_name
            )));
        }

        root.children
            .iter()
            .filter_map(|child| match child {
                ASTNodeOrToken::Node(node) if node.rule_name == "statement" => Some(node),
                _ => None,
            })
            .map(|statement| self.compile_statement(statement))
            .collect()
    }

    fn compile_statement(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let expr = node
            .children
            .iter()
            .find_map(as_node)
            .ok_or_else(|| CompileError::new("statement has no expression"))?;
        let inner = self.compile_node(expr)?;
        if !self.options.wrap_terminators {
            return Ok(inner);
        }

        let terminator = node.children.iter().find_map(as_token);
        let head = if terminator.is_some_and(|token| token_type_name(token) == "DOLLAR") {
            SUPPRESS
        } else {
            DISPLAY
        };
        Ok(apply(sym(head), vec![inner]))
    }

    fn compile_child(&self, child: &ASTNodeOrToken) -> Result<IRNode, CompileError> {
        match child {
            ASTNodeOrToken::Node(node) => self.compile_node(node),
            ASTNodeOrToken::Token(token) => self.compile_token(token),
        }
    }

    fn compile_node(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        match unwrap_node(node) {
            Unwrapped::Token(token) => self.compile_token(token),
            Unwrapped::Node(node) => match node.rule_name.as_str() {
                "statement" => self.compile_statement(node),
                "expression" => self.compile_first_node(node),
                "assign" => self.compile_assign(node),
                "logical_or" => self.compile_logical_chain(node, OR),
                "logical_and" => self.compile_logical_chain(node, AND),
                "logical_not" => self.compile_logical_not(node),
                "comparison" => self.compile_comparison(node),
                "additive" | "multiplicative" => self.compile_binary_chain(node),
                "unary" => self.compile_unary(node),
                "power" => self.compile_power(node),
                "postfix" => self.compile_postfix(node),
                "atom" => self.compile_first(node),
                "group" => self.compile_delimited_single(node),
                "list" => self.compile_list(node),
                "if_expr" => self.compile_if(node),
                "while_expr" => self.compile_while(node),
                "for_expr" => self.compile_first_node(node),
                "for_each_expr" => self.compile_for_each(node),
                "for_range_expr" => self.compile_for_range(node),
                "block_expr" => self.compile_block(node),
                "return_expr" => self.compile_return(node),
                "arglist" => Err(CompileError::new(
                    "arglist cannot be compiled as a scalar expression",
                )),
                other => Err(CompileError::new(format!("no compiler for rule {other}"))),
            },
        }
    }

    fn compile_token(&self, token: &Token) -> Result<IRNode, CompileError> {
        match token_type_name(token) {
            "NUMBER" => parse_number(&token.value),
            "NAME" => Ok(sym(&token.value)),
            "STRING" => Ok(str_node(&token.value)),
            "KEYWORD" if token.value == "true" => Ok(sym("True")),
            "KEYWORD" if token.value == "false" => Ok(sym("False")),
            other => Err(CompileError::new(format!(
                "unexpected token {other}={:?}",
                token.value
            ))),
        }
    }

    fn compile_first(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let child = node
            .children
            .first()
            .ok_or_else(|| CompileError::new(format!("{} has no children", node.rule_name)))?;
        self.compile_child(child)
    }

    fn compile_first_node(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let child = node
            .children
            .iter()
            .find_map(as_node)
            .ok_or_else(|| CompileError::new(format!("{} has no AST child", node.rule_name)))?;
        self.compile_node(child)
    }

    fn compile_delimited_single(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let child = node.children.iter().find_map(as_node).ok_or_else(|| {
            CompileError::new(format!("{} has no inner expression", node.rule_name))
        })?;
        self.compile_node(child)
    }

    fn compile_assign(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let op_index = node.children.iter().position(|child| {
            as_token(child)
                .is_some_and(|token| matches!(token_type_name(token), "COLON" | "COLONEQ"))
        });
        let Some(op_index) = op_index else {
            return self.compile_first_node(node);
        };

        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(CompileError::new("malformed assign node"));
        }
        let lhs = self.compile_child(&node.children[op_index - 1])?;
        let rhs = self.compile_child(&node.children[op_index + 1])?;
        let op = as_token(&node.children[op_index])
            .map(token_type_name)
            .ok_or_else(|| CompileError::new("assign op must be a token"))?;

        if op == "COLONEQ" {
            if let IRNode::Apply(apply_node) = &lhs {
                if matches!(&apply_node.head, IRNode::Symbol(_)) {
                    return Ok(apply(
                        sym(DEFINE),
                        vec![
                            apply_node.head.clone(),
                            apply(sym(LIST), apply_node.args.clone()),
                            rhs,
                        ],
                    ));
                }
            }
            return Ok(apply(sym(DEFINE), vec![lhs, apply(sym(LIST), vec![]), rhs]));
        }
        Ok(apply(sym(ASSIGN), vec![lhs, rhs]))
    }

    fn compile_logical_chain(
        &self,
        node: &GrammarASTNode,
        head: &str,
    ) -> Result<IRNode, CompileError> {
        let operands: Result<Vec<_>, _> = node
            .children
            .iter()
            .filter_map(as_node)
            .map(|child| self.compile_node(child))
            .collect();
        let operands = operands?;
        if operands.len() == 1 {
            Ok(operands.into_iter().next().unwrap())
        } else {
            Ok(apply(sym(head), operands))
        }
    }

    fn compile_logical_not(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let has_not = node
            .children
            .iter()
            .any(|child| as_token(child).is_some_and(|token| token.value == "not"));
        if !has_not {
            return self.compile_first_node(node);
        }
        let child = node
            .children
            .iter()
            .find_map(as_node)
            .ok_or_else(|| CompileError::new("not expression missing operand"))?;
        Ok(apply(sym(NOT), vec![self.compile_node(child)?]))
    }

    fn compile_comparison(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let op_index = node.children.iter().position(|child| {
            as_token(child).is_some_and(|token| comparison_head(token_type_name(token)).is_some())
        });
        let Some(op_index) = op_index else {
            return self.compile_first_node(node);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(CompileError::new("malformed comparison node"));
        }
        let op = as_token(&node.children[op_index])
            .and_then(|token| comparison_head(token_type_name(token)))
            .ok_or_else(|| CompileError::new("unknown comparison op"))?;
        Ok(apply(
            sym(op),
            vec![
                self.compile_child(&node.children[op_index - 1])?,
                self.compile_child(&node.children[op_index + 1])?,
            ],
        ))
    }

    fn compile_binary_chain(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| CompileError::new("empty binary chain"))?;
        let mut result = self.compile_child(first)?;
        while let Some(op_child) = children.next() {
            let op = as_token(op_child)
                .and_then(|token| binary_head(token_type_name(token)))
                .ok_or_else(|| CompileError::new("unknown binary op"))?;
            let rhs = children
                .next()
                .ok_or_else(|| CompileError::new("binary op missing rhs"))?;
            result = apply(sym(op), vec![result, self.compile_child(rhs)?]);
        }
        Ok(result)
    }

    fn compile_unary(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        if node.children.len() == 1 {
            return self.compile_child(&node.children[0]);
        }
        let op = as_token(&node.children[0])
            .map(token_type_name)
            .ok_or_else(|| CompileError::new("unary op must be a token"))?;
        let value = self.compile_child(
            node.children
                .get(1)
                .ok_or_else(|| CompileError::new("unary missing operand"))?,
        )?;
        if op == "MINUS" {
            Ok(apply(sym(NEG), vec![value]))
        } else {
            Ok(value)
        }
    }

    fn compile_power(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        if node.children.len() == 1 {
            return self.compile_child(&node.children[0]);
        }
        if node.children.len() != 3 {
            return Err(CompileError::new("malformed power node"));
        }
        Ok(apply(
            sym(POW),
            vec![
                self.compile_child(&node.children[0])?,
                self.compile_child(&node.children[2])?,
            ],
        ))
    }

    fn compile_postfix(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let mut result = self.compile_child(
            node.children
                .first()
                .ok_or_else(|| CompileError::new("postfix has no base"))?,
        )?;

        let mut i = 1;
        while i < node.children.len() {
            let Some(token) = as_token(&node.children[i]) else {
                i += 1;
                continue;
            };
            if token_type_name(token) != "LPAREN" {
                i += 1;
                continue;
            }

            let args = node
                .children
                .get(i + 1)
                .and_then(as_node)
                .filter(|child| child.rule_name == "arglist")
                .map(|arglist| self.compile_arglist(arglist))
                .transpose()?
                .unwrap_or_default();
            result = apply(canonical_call_head(result), args);
            i += 1;
        }
        Ok(result)
    }

    fn compile_list(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let mut args = Vec::new();
        for child in node.children.iter().filter_map(as_node) {
            if child.rule_name == "arglist" {
                args.extend(self.compile_arglist(child)?);
            }
        }
        Ok(apply(sym(LIST), args))
    }

    fn compile_arglist(&self, node: &GrammarASTNode) -> Result<Vec<IRNode>, CompileError> {
        node.children
            .iter()
            .filter_map(as_node)
            .map(|child| self.compile_node(child))
            .collect()
    }

    fn compile_if(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let expressions: Result<Vec<_>, _> = node
            .children
            .iter()
            .filter_map(as_node)
            .map(|child| self.compile_node(child))
            .collect();
        let expressions = expressions?;
        if expressions.len() < 2 {
            return Err(CompileError::new(
                "if expression needs condition and then branch",
            ));
        }

        let has_else = expressions.len() % 2 == 1;
        let mut fallback = if has_else {
            expressions.last().unwrap().clone()
        } else {
            sym("False")
        };
        let pair_limit = if has_else {
            expressions.len() - 1
        } else {
            expressions.len()
        };
        let mut i = pair_limit;
        while i >= 2 {
            i -= 2;
            fallback = apply(
                sym(IF),
                vec![expressions[i].clone(), expressions[i + 1].clone(), fallback],
            );
        }
        Ok(fallback)
    }

    fn compile_while(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let expressions: Result<Vec<_>, _> = node
            .children
            .iter()
            .filter_map(as_node)
            .map(|child| self.compile_node(child))
            .collect();
        let expressions = expressions?;
        if expressions.len() != 2 {
            return Err(CompileError::new(
                "while expression needs condition and body",
            ));
        }
        Ok(apply(sym(WHILE), expressions))
    }

    fn compile_for_each(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let variable = node.children.iter().find_map(|child| {
            as_token(child)
                .filter(|token| token_type_name(token) == "NAME")
                .map(|token| token.value.clone())
        });
        let expressions: Result<Vec<_>, _> = node
            .children
            .iter()
            .filter_map(as_node)
            .map(|child| self.compile_node(child))
            .collect();
        let expressions = expressions?;
        if variable.is_none() || expressions.len() != 2 {
            return Err(CompileError::new("for-each expression malformed"));
        }
        Ok(apply(
            sym(FOR_EACH),
            vec![
                sym(variable.unwrap()),
                expressions[0].clone(),
                expressions[1].clone(),
            ],
        ))
    }

    fn compile_for_range(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let variable = node.children.iter().find_map(|child| {
            as_token(child)
                .filter(|token| token_type_name(token) == "NAME")
                .map(|token| token.value.clone())
        });
        let expressions: Result<Vec<_>, _> = node
            .children
            .iter()
            .filter_map(as_node)
            .map(|child| self.compile_node(child))
            .collect();
        let expressions = expressions?;
        let Some(variable) = variable else {
            return Err(CompileError::new("for-range expression missing variable"));
        };
        if expressions.len() < 2 {
            return Err(CompileError::new("for-range expression malformed"));
        }

        let (start, step, end, body) = match expressions.as_slice() {
            [end, body] => (int(1), int(1), end.clone(), body.clone()),
            [start, end, body] => (start.clone(), int(1), end.clone(), body.clone()),
            [start, step, end, body, ..] => {
                (start.clone(), step.clone(), end.clone(), body.clone())
            }
            _ => unreachable!(),
        };
        Ok(apply(
            sym(FOR_RANGE),
            vec![sym(variable), start, step, end, body],
        ))
    }

    fn compile_block(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let Some(args_node) = node
            .children
            .iter()
            .find_map(as_node)
            .filter(|child| child.rule_name == "arglist")
        else {
            return Ok(apply(sym(BLOCK), vec![apply(sym(LIST), vec![])]));
        };
        let args = self.compile_arglist(args_node)?;
        if args.first().is_some_and(is_list_apply) {
            Ok(apply(sym(BLOCK), args))
        } else {
            let mut with_locals = vec![apply(sym(LIST), vec![])];
            with_locals.extend(args);
            Ok(apply(sym(BLOCK), with_locals))
        }
    }

    fn compile_return(&self, node: &GrammarASTNode) -> Result<IRNode, CompileError> {
        let child = node
            .children
            .iter()
            .find_map(as_node)
            .ok_or_else(|| CompileError::new("return expression missing value"))?;
        Ok(apply(sym(RETURN), vec![self.compile_node(child)?]))
    }
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

fn unwrap_node(mut node: &GrammarASTNode) -> Unwrapped<'_> {
    loop {
        if node.children.len() != 1 {
            return Unwrapped::Node(node);
        }
        match &node.children[0] {
            ASTNodeOrToken::Node(child) => node = child,
            ASTNodeOrToken::Token(token) => return Unwrapped::Token(token),
        }
    }
}

fn as_node(child: &ASTNodeOrToken) -> Option<&GrammarASTNode> {
    match child {
        ASTNodeOrToken::Node(node) => Some(node),
        ASTNodeOrToken::Token(_) => None,
    }
}

fn as_token(child: &ASTNodeOrToken) -> Option<&Token> {
    match child {
        ASTNodeOrToken::Node(_) => None,
        ASTNodeOrToken::Token(token) => Some(token),
    }
}

fn token_type_name(token: &Token) -> &str {
    token.effective_type_name()
}

fn parse_number(text: &str) -> Result<IRNode, CompileError> {
    if text.contains('.') || text.contains('e') || text.contains('E') {
        let value = text
            .parse::<f64>()
            .map_err(|err| CompileError::new(format!("invalid float literal {text:?}: {err}")))?;
        Ok(flt(value))
    } else {
        let value = text
            .parse::<i64>()
            .map_err(|err| CompileError::new(format!("invalid integer literal {text:?}: {err}")))?;
        Ok(int(value))
    }
}

fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "STAR" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

fn comparison_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "EQ" => Some(EQUAL),
        "HASH" => Some(NOT_EQUAL),
        "LT" => Some(LESS),
        "GT" => Some(GREATER),
        "LEQ" => Some(LESS_EQUAL),
        "GEQ" => Some(GREATER_EQUAL),
        _ => None,
    }
}

fn canonical_call_head(head: IRNode) -> IRNode {
    if let IRNode::Symbol(name) = &head {
        if let Some(canonical) = standard_function(name) {
            return sym(canonical);
        }
    }
    head
}

fn standard_function(name: &str) -> Option<&'static str> {
    match name {
        "diff" => Some(D),
        "integrate" => Some(INTEGRATE),
        "sin" => Some(SIN),
        "cos" => Some(COS),
        "tan" => Some(TAN),
        "asin" => Some(ASIN),
        "acos" => Some(ACOS),
        "atan" => Some(ATAN),
        "sinh" => Some(SINH),
        "cosh" => Some(COSH),
        "tanh" => Some(TANH),
        "asinh" => Some(ASINH),
        "acosh" => Some(ACOSH),
        "atanh" => Some(ATANH),
        "coth" => Some("Coth"),
        "sech" => Some("Sech"),
        "csch" => Some("Csch"),
        "log" => Some(LOG),
        "exp" => Some(EXP),
        "sqrt" => Some(SQRT),
        "sum" => Some("Sum"),
        "product" => Some("Product"),
        "factor" => Some("Factor"),
        "solve" => Some("Solve"),
        "simplify" => Some("Simplify"),
        "subst" => Some("Subst"),
        "assume" => Some("Assume"),
        "forget" => Some("Forget"),
        "is" => Some("Is"),
        "sign" => Some("Sign"),
        _ => None,
    }
}

fn is_list_apply(node: &IRNode) -> bool {
    matches!(
        node,
        IRNode::Apply(apply_node)
            if matches!(&apply_node.head, IRNode::Symbol(name) if name == LIST)
    )
}
