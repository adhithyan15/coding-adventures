//! A small, fail-closed TypeScript-to-WIR bridge for scalar bit helpers.
//!
//! The repository's TypeScript parser provides a real, versioned grammar AST,
//! while WIR already supplies fixed-width types, a checker, a symbolic
//! executor, and an existing SAT-backed prover. This crate connects those
//! pieces for one intentionally narrow input shape. It lowers the operator
//! tokens found in the AST; it never substitutes a hard-coded correct formula
//! when the source contains a mutant.

pub mod capabilities;

use coding_adventures_typescript_lexer::tokenize_typescript;
use coding_adventures_typescript_parser::parse_typescript;
use coding_adventures_warrant_ir::{
    BasicBlock, BinaryOp, BlockId, Function, Label, Program, TypeChecker, UnaryOp, VarId, WirExpr,
    WirTerminator, WirType,
};
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::fmt;

/// Grammar selected by the first source-bound proof consumer.
pub const TYPESCRIPT_VERSION: &str = "ts5.8";
/// The admitted helper file is deliberately small and checked before parsing.
pub const MAX_SOURCE_BYTES: usize = 4_096;
/// Preflight token cap; this check happens before the grammar parser builds an AST.
pub const MAX_TOKENS: usize = 128;
/// Visitor budget after parsing; it bounds AST inspection, not parser allocation.
pub const MAX_AST_NODES: usize = 512;
/// Bound on the WIR expression nodes emitted from the admitted helpers.
pub const MAX_WIR_EXPR_NODES: usize = 64;

/// A refusal from the bounded TypeScript source adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    SourceTooLarge { actual: usize, limit: usize },
    InvalidUtf8,
    TooManyTokens { actual: usize, limit: usize },
    Parse(String),
    TooManyAstNodes { actual: usize, limit: usize },
    UnsupportedAst(String),
    TypeCheck(String),
    TooManyWirNodes { limit: usize },
    InvalidNotContext,
    RestrictedProductionLineTerminator,
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { actual, limit } => {
                write!(f, "source has {actual} bytes; limit is {limit}")
            }
            Self::InvalidUtf8 => f.write_str("source is not UTF-8"),
            Self::TooManyTokens { actual, limit } => {
                write!(f, "source has {actual} tokens; limit is {limit}")
            }
            Self::Parse(message) => write!(f, "TypeScript parse refused: {message}"),
            Self::TooManyAstNodes { actual, limit } => {
                write!(f, "AST has {actual} nodes; limit is {limit}")
            }
            Self::UnsupportedAst(message) => write!(f, "unsupported TypeScript AST: {message}"),
            Self::TypeCheck(message) => write!(f, "lowered WIR failed type checking: {message}"),
            Self::TooManyWirNodes { limit } => write!(f, "WIR expression limit {limit} exceeded"),
            Self::InvalidNotContext => f.write_str("BigInt NOT is not under a finite mask"),
            Self::RestrictedProductionLineTerminator => {
                f.write_str("line terminator after return changes JavaScript semantics")
            }
        }
    }
}

impl std::error::Error for BridgeError {}

/// Parse the exact source bytes and lower their three exported helpers to WIR.
///
/// The returned program contains the operators actually present in the AST.
/// For example, replacing `!==` with `===` produces `BinaryOp::Eq`; callers
/// must prove or falsify that WIR instead of asking this function to repair it.
pub fn lower_typescript_bytes(source_bytes: &[u8]) -> Result<Program, BridgeError> {
    if source_bytes.len() > MAX_SOURCE_BYTES {
        return Err(BridgeError::SourceTooLarge {
            actual: source_bytes.len(),
            limit: MAX_SOURCE_BYTES,
        });
    }
    let source = std::str::from_utf8(source_bytes).map_err(|_| BridgeError::InvalidUtf8)?;
    let tokens = tokenize_typescript(source, TYPESCRIPT_VERSION)
        .map_err(|message| BridgeError::Parse(message.to_string()))?;
    if tokens.len() > MAX_TOKENS {
        return Err(BridgeError::TooManyTokens {
            actual: tokens.len(),
            limit: MAX_TOKENS,
        });
    }

    // The TypeScript parser installs its measured ts5.8 rule-depth guard here.
    let ast = parse_typescript(source, TYPESCRIPT_VERSION)
        .map_err(|message| BridgeError::Parse(message.to_string()))?;
    ensure_ast_budget(&ast)?;
    let program = lower_program(&ast, source)?;
    TypeChecker::new(&program)
        .check_program()
        .map_err(|error| BridgeError::TypeCheck(format!("{error:?}")))?;
    Ok(program)
}

fn ensure_ast_budget(ast: &GrammarASTNode) -> Result<(), BridgeError> {
    let ast_nodes = count_ast_nodes(ast, MAX_AST_NODES + 1);
    if ast_nodes > MAX_AST_NODES {
        Err(BridgeError::TooManyAstNodes {
            actual: ast_nodes,
            limit: MAX_AST_NODES,
        })
    } else {
        Ok(())
    }
}

fn count_ast_nodes(node: &GrammarASTNode, stop_after: usize) -> usize {
    let mut count = 1;
    for child in &node.children {
        if let ASTNodeOrToken::Node(child) = child {
            count += count_ast_nodes(child, stop_after.saturating_sub(count));
            if count >= stop_after {
                return count;
            }
        }
    }
    count
}

fn node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(child) => Some(child),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn token_child(node: &GrammarASTNode, index: usize) -> Option<&Token> {
    match node.children.get(index) {
        Some(ASTNodeOrToken::Token(token)) => Some(token),
        _ => None,
    }
}

fn node_child(node: &GrammarASTNode, index: usize) -> Option<&GrammarASTNode> {
    match node.children.get(index) {
        Some(ASTNodeOrToken::Node(child)) => Some(child),
        _ => None,
    }
}

fn expect_token(node: &GrammarASTNode, index: usize, value: &str) -> Result<(), BridgeError> {
    if token_child(node, index).is_some_and(|token| token.value == value) {
        Ok(())
    } else {
        Err(BridgeError::UnsupportedAst(format!(
            "{} child {index} must be token {value:?}",
            node.rule_name
        )))
    }
}

fn expect_rule<'a>(
    node: &'a GrammarASTNode,
    index: usize,
    rule_name: &str,
) -> Result<&'a GrammarASTNode, BridgeError> {
    let child = node_child(node, index).ok_or_else(|| {
        BridgeError::UnsupportedAst(format!(
            "{} child {index} must be {rule_name}",
            node.rule_name
        ))
    })?;
    if child.rule_name == rule_name {
        Ok(child)
    } else {
        Err(BridgeError::UnsupportedAst(format!(
            "{} child {index} must be {rule_name}, found {}",
            node.rule_name, child.rule_name
        )))
    }
}

fn only_type(node: &GrammarASTNode, expected: &str) -> Result<(), BridgeError> {
    let chain = [
        ("type_expression", "conditional_type"),
        ("conditional_type", "union_type"),
        ("union_type", "intersection_type"),
        ("intersection_type", "array_type"),
        ("array_type", "primary_type"),
        ("primary_type", "predefined_type"),
    ];
    let mut current = node;
    for (parent_rule, child_rule) in chain {
        if current.rule_name != parent_rule || current.children.len() != 1 {
            return Err(BridgeError::UnsupportedAst(format!(
                "type must be the predefined type {expected}, found {}",
                current.rule_name
            )));
        }
        current = expect_rule(current, 0, child_rule)?;
    }
    let token = token_child(current, 0).ok_or_else(|| {
        BridgeError::UnsupportedAst(format!("type must be predefined {expected}"))
    })?;
    if current.children.len() == 1 && token.type_ == TokenType::Name && token.value == expected {
        Ok(())
    } else {
        Err(BridgeError::UnsupportedAst(format!(
            "type must be the predefined name {expected}"
        )))
    }
}

#[derive(Clone, Copy)]
enum HelperRole {
    BitIsSet,
    SetBit,
    ClearBit,
}

impl HelperRole {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::BitIsSet,
            1 => Self::SetBit,
            _ => Self::ClearBit,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::BitIsSet => "bit_is_set",
            Self::SetBit => "set_bit",
            Self::ClearBit => "clear_bit",
        }
    }

    fn return_type(self) -> &'static str {
        match self {
            Self::BitIsSet => "boolean",
            Self::SetBit | Self::ClearBit => "bigint",
        }
    }

    fn wir_return_type(self) -> WirType {
        match self {
            Self::BitIsSet => WirType::Bool,
            Self::SetBit | Self::ClearBit => WirType::U64,
        }
    }
}

fn lower_program(ast: &GrammarASTNode, source: &str) -> Result<Program, BridgeError> {
    if ast.rule_name != "program" || ast.children.len() != 3 {
        return Err(BridgeError::UnsupportedAst(
            "program must contain exactly three exported helper declarations".to_string(),
        ));
    }
    let mut functions = Vec::with_capacity(3);
    for (index, source_element) in node_children(ast).into_iter().enumerate() {
        if source_element.rule_name != "source_element" || source_element.children.len() != 1 {
            return Err(BridgeError::UnsupportedAst(
                "top level may contain only the three helper exports".to_string(),
            ));
        }
        let export = expect_rule(source_element, 0, "export_declaration")?;
        if export.children.len() != 2 {
            return Err(BridgeError::UnsupportedAst(
                "each export must be one function declaration".to_string(),
            ));
        }
        expect_token(export, 0, "export")?;
        let function = expect_rule(export, 1, "function_declaration")?;
        functions.push(lower_function(
            function,
            HelperRole::from_index(index),
            source,
        )?);
    }
    Ok(Program::new(functions))
}

fn lower_function(
    function: &GrammarASTNode,
    role: HelperRole,
    source: &str,
) -> Result<Function, BridgeError> {
    if function.rule_name != "function_declaration" || function.children.len() != 10 {
        return Err(BridgeError::UnsupportedAst(format!(
            "{} must use one typed function declaration",
            role.name()
        )));
    }
    expect_token(function, 0, "function")?;
    let name = token_child(function, 1)
        .ok_or_else(|| BridgeError::UnsupportedAst("function name is not a token".to_string()))?;
    if name.type_ != TokenType::Name || name.value != role.name() {
        return Err(BridgeError::UnsupportedAst(format!(
            "expected function {}, found {}",
            role.name(),
            name.value
        )));
    }
    expect_token(function, 2, "(")?;
    let parameter_list = expect_rule(function, 3, "typed_parameter_list")?;
    expect_token(function, 4, ")")?;
    expect_token(function, 5, ":")?;
    only_type(
        expect_rule(function, 6, "type_expression")?,
        role.return_type(),
    )?;
    expect_token(function, 7, "{")?;
    let body = expect_rule(function, 8, "function_body")?;
    expect_token(function, 9, "}")?;
    if parameter_list.children.len() != 3 {
        return Err(BridgeError::UnsupportedAst(
            "helper requires exactly mask and slot parameters".to_string(),
        ));
    }
    let mask_param = expect_rule(parameter_list, 0, "typed_parameter")?;
    expect_token(parameter_list, 1, ",")?;
    let slot_param = expect_rule(parameter_list, 2, "typed_parameter")?;
    check_parameter(mask_param, "mask", "bigint")?;
    check_parameter(slot_param, "slot", "number")?;

    if body.children.len() != 1 {
        return Err(BridgeError::UnsupportedAst(
            "function body must contain exactly one return".to_string(),
        ));
    }
    let source_element = expect_rule(body, 0, "source_element")?;
    let statement = expect_rule(source_element, 0, "statement")?;
    let ret = expect_rule(statement, 0, "return_statement")?;
    if ret.children.len() != 3 {
        return Err(BridgeError::UnsupportedAst(
            "helper must return exactly one expression".to_string(),
        ));
    }
    expect_token(ret, 0, "return")?;
    let expression = expect_rule(ret, 1, "expression")?;
    expect_token(ret, 2, ";")?;
    let return_token = token_child(ret, 0).expect("return statement was checked");
    let first = first_token(expression).ok_or_else(|| {
        BridgeError::UnsupportedAst("return expression has no source token".to_string())
    })?;
    let return_end = source_offset(source, return_token.line, return_token.column)
        .and_then(|offset| offset.checked_add(return_token.value.len()))
        .ok_or_else(|| BridgeError::UnsupportedAst("return token has no source span".into()))?;
    let expression_start = source_offset(source, first.line, first.column).ok_or_else(|| {
        BridgeError::UnsupportedAst("return expression has no source span".into())
    })?;
    if source
        .get(return_end..expression_start)
        .is_none_or(|trivia| trivia.chars().any(is_line_terminator))
    {
        return Err(BridgeError::RestrictedProductionLineTerminator);
    }

    let mut budget = WirBudget { count: 0 };
    let value = lower_expression(expression, &mut budget)?;
    ensure_not_is_masked(&value, false)?;
    Ok(Function {
        name: role.name().to_string(),
        params: vec![
            (VarId::from("mask"), WirType::U64, Label::PUBLIC),
            (VarId::from("slot"), WirType::U8, Label::PUBLIC),
        ],
        return_type: role.wir_return_type(),
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: WirTerminator::Return(Some(value)),
        }],
        ranking_metric: None,
    })
}

fn first_token(node: &GrammarASTNode) -> Option<&Token> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(token) => return Some(token),
            ASTNodeOrToken::Node(child) => {
                if let Some(token) = first_token(child) {
                    return Some(token);
                }
            }
        }
    }
    None
}

/// Convert the parser's one-based line/character-column pair back to a byte
/// offset. The source remains available because the AST omits trivia, while
/// JavaScript's `return` production assigns meaning to a line break in trivia.
fn source_offset(source: &str, target_line: usize, target_column: usize) -> Option<usize> {
    if target_line == 0 || target_column == 0 {
        return None;
    }
    let mut byte_start = 0;
    for (line_index, segment) in source.split_inclusive('\n').enumerate() {
        if line_index + 1 == target_line {
            let line = segment.strip_suffix('\n').unwrap_or(segment);
            let character_index = target_column - 1;
            let within_line = line
                .char_indices()
                .nth(character_index)
                .map(|(offset, _)| offset)
                .or_else(|| (line.chars().count() == character_index).then_some(line.len()))?;
            return Some(byte_start + within_line);
        }
        byte_start += segment.len();
    }
    None
}

fn is_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn check_parameter(
    parameter: &GrammarASTNode,
    expected_name: &str,
    expected_type: &str,
) -> Result<(), BridgeError> {
    if parameter.rule_name != "typed_parameter" || parameter.children.len() != 3 {
        return Err(BridgeError::UnsupportedAst(
            "parameters must be simple named typed parameters".to_string(),
        ));
    }
    let name = token_child(parameter, 0)
        .ok_or_else(|| BridgeError::UnsupportedAst("parameter name is not a token".to_string()))?;
    if name.type_ != TokenType::Name || name.value != expected_name {
        return Err(BridgeError::UnsupportedAst(format!(
            "expected parameter {expected_name}, found {}",
            name.value
        )));
    }
    expect_token(parameter, 1, ":")?;
    only_type(expect_rule(parameter, 2, "type_expression")?, expected_type)
}

#[derive(Default)]
struct WirBudget {
    count: usize,
}

impl WirBudget {
    fn take(&mut self) -> Result<(), BridgeError> {
        self.count += 1;
        if self.count > MAX_WIR_EXPR_NODES {
            Err(BridgeError::TooManyWirNodes {
                limit: MAX_WIR_EXPR_NODES,
            })
        } else {
            Ok(())
        }
    }
}

fn lower_expression(node: &GrammarASTNode, budget: &mut WirBudget) -> Result<WirExpr, BridgeError> {
    if node.rule_name == "primary_expression" {
        return lower_primary(node, budget);
    }
    if node.rule_name == "call_expression" {
        return lower_bigint_call(node, budget);
    }
    if node.rule_name == "unary_expression"
        && node.children.len() == 2
        && token_child(node, 0).is_some_and(|token| token.value == "~")
    {
        expect_operator(token_child(node, 0).expect("checked above"), "~", "TILDE")?;
        let operand = node_child(node, 1).ok_or_else(|| {
            BridgeError::UnsupportedAst("unary NOT requires one expression".to_string())
        })?;
        budget.take()?;
        return Ok(WirExpr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(lower_expression(operand, budget)?),
        });
    }

    if node.children.len() == 3
        && node_child(node, 0).is_some()
        && token_child(node, 1).is_some()
        && node_child(node, 2).is_some()
    {
        let operator = token_child(node, 1).expect("checked above");
        let (op, expected_rule, expected_token) = match operator.value.as_str() {
            "&" => (BinaryOp::BitAnd, "bitwise_and_expression", "AMPERSAND"),
            "|" => (BinaryOp::BitOr, "bitwise_or_expression", "PIPE"),
            "<<" => (BinaryOp::Shl, "shift_expression", "LEFT_SHIFT"),
            "!==" => (BinaryOp::Ne, "equality_expression", "STRICT_NOT_EQUALS"),
            "===" => (BinaryOp::Eq, "equality_expression", "STRICT_EQUALS"),
            _ => {
                return Err(BridgeError::UnsupportedAst(format!(
                    "operator {:?} is outside the admitted subset",
                    operator.value
                )))
            }
        };
        if node.rule_name != expected_rule {
            return Err(BridgeError::UnsupportedAst(format!(
                "operator {:?} is not in grammar rule {}",
                operator.value, expected_rule
            )));
        }
        expect_operator(operator, &operator.value, expected_token)?;
        let lhs = lower_expression(node_child(node, 0).unwrap(), budget)?;
        let rhs = lower_expression(node_child(node, 2).unwrap(), budget)?;
        if op == BinaryOp::Shl && !is_exact_slot_shift(&lhs, &rhs) {
            return Err(BridgeError::UnsupportedAst(
                "only literal 1n shifted by the directly converted slot is admitted".to_string(),
            ));
        }
        if matches!(op, BinaryOp::Eq | BinaryOp::Ne)
            && rhs
                != (WirExpr::ConstInt {
                    val: 0,
                    ty: WirType::U64,
                })
        {
            return Err(BridgeError::UnsupportedAst(
                "equality in this subset must compare against 0n".to_string(),
            ));
        }
        budget.take()?;
        return Ok(WirExpr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        });
    }

    if is_transparent_expression_rule(&node.rule_name) && node.children.len() == 1 {
        if let Some(child) = node_child(node, 0) {
            return lower_expression(child, budget);
        }
    }
    Err(BridgeError::UnsupportedAst(format!(
        "{} has a shape outside the scalar expression subset",
        node.rule_name
    )))
}

fn is_transparent_expression_rule(rule: &str) -> bool {
    matches!(
        rule,
        "expression"
            | "assignment_expression"
            | "conditional_expression"
            | "nullish_coalescing_expression"
            | "logical_or_expression"
            | "logical_and_expression"
            | "bitwise_or_expression"
            | "bitwise_xor_expression"
            | "bitwise_and_expression"
            | "equality_expression"
            | "relational_expression"
            | "shift_expression"
            | "additive_expression"
            | "multiplicative_expression"
            | "exponentiation_expression"
            | "unary_expression"
            | "postfix_expression"
            | "left_hand_side_expression"
            | "optional_chain_expression"
            | "member_expression"
    )
}

fn is_exact_slot_shift(lhs: &WirExpr, rhs: &WirExpr) -> bool {
    matches!(
        lhs,
        WirExpr::ConstInt {
            val: 1,
            ty: WirType::U64
        }
    ) && matches!(
        rhs,
        WirExpr::ZExt { expr, target_ty: WirType::U64 }
            if matches!(expr.as_ref(), WirExpr::Var(var) if var.0 == "slot")
    )
}

fn lower_primary(node: &GrammarASTNode, budget: &mut WirBudget) -> Result<WirExpr, BridgeError> {
    if node.children.len() == 1 {
        if let Some(token) = token_child(node, 0) {
            budget.take()?;
            return match token.value.as_str() {
                "mask" if token.type_ == TokenType::Name => Ok(WirExpr::Var(VarId::from("mask"))),
                "slot" if token.type_ == TokenType::Name => Ok(WirExpr::Var(VarId::from("slot"))),
                "0n" if token.type_ == TokenType::Name
                    && token.type_name.as_deref() == Some("BIGINT") =>
                {
                    Ok(WirExpr::ConstInt {
                        val: 0,
                        ty: WirType::U64,
                    })
                }
                "1n" if token.type_ == TokenType::Name
                    && token.type_name.as_deref() == Some("BIGINT") =>
                {
                    Ok(WirExpr::ConstInt {
                        val: 1,
                        ty: WirType::U64,
                    })
                }
                _ => Err(BridgeError::UnsupportedAst(format!(
                    "primary token {:?} is not an admitted parameter or BigInt literal",
                    token.value
                ))),
            };
        }
    }
    if node.children.len() == 3
        && token_child(node, 0).is_some_and(|token| token.value == "(")
        && token_child(node, 2).is_some_and(|token| token.value == ")")
    {
        return lower_expression(expect_rule(node, 1, "expression")?, budget);
    }
    Err(BridgeError::UnsupportedAst(
        "primary expression is not a parameter, bounded literal, or parenthesized expression"
            .to_string(),
    ))
}

fn lower_bigint_call(
    node: &GrammarASTNode,
    budget: &mut WirBudget,
) -> Result<WirExpr, BridgeError> {
    if node.children.len() != 2 {
        return Err(BridgeError::UnsupportedAst(
            "only direct BigInt(slot) calls are admitted".to_string(),
        ));
    }
    let callee = expect_rule(node, 0, "member_expression")?;
    let name = simple_primary_name(callee)?;
    if name != "BigInt" {
        return Err(BridgeError::UnsupportedAst(
            "the only admitted call is the unshadowed BigInt intrinsic".to_string(),
        ));
    }
    let arguments = expect_rule(node, 1, "arguments")?;
    if arguments.children.len() != 3 {
        return Err(BridgeError::UnsupportedAst(
            "BigInt call must have exactly one positional argument".to_string(),
        ));
    }
    expect_token(arguments, 0, "(")?;
    let argument_list = expect_rule(arguments, 1, "argument_list")?;
    expect_token(arguments, 2, ")")?;
    if argument_list.children.len() != 1 {
        return Err(BridgeError::UnsupportedAst(
            "BigInt call must have exactly one argument".to_string(),
        ));
    }
    let argument = node_child(argument_list, 0).ok_or_else(|| {
        BridgeError::UnsupportedAst("BigInt argument must be an expression".to_string())
    })?;
    let lowered = lower_expression(argument, budget)?;
    if lowered != WirExpr::Var(VarId::from("slot")) {
        return Err(BridgeError::UnsupportedAst(
            "BigInt may only convert the slot parameter directly".to_string(),
        ));
    }
    budget.take()?;
    Ok(WirExpr::ZExt {
        expr: Box::new(lowered),
        target_ty: WirType::U64,
    })
}

fn simple_primary_name(node: &GrammarASTNode) -> Result<&str, BridgeError> {
    let mut current = node;
    loop {
        if current.rule_name == "primary_expression" {
            let token = token_child(current, 0).ok_or_else(|| {
                BridgeError::UnsupportedAst("BigInt callee must be a plain identifier".to_string())
            })?;
            if current.children.len() != 1 || token.type_ != TokenType::Name {
                return Err(BridgeError::UnsupportedAst(
                    "BigInt callee must be a plain identifier".to_string(),
                ));
            }
            return Ok(&token.value);
        }
        if current.children.len() == 1 {
            if let Some(child) = node_child(current, 0) {
                current = child;
                continue;
            }
        }
        return Err(BridgeError::UnsupportedAst(
            "BigInt callee must be an unqualified global name".to_string(),
        ));
    }
}

fn expect_operator(token: &Token, value: &str, type_name: &str) -> Result<(), BridgeError> {
    if token.value == value && token.type_name.as_deref() == Some(type_name) {
        Ok(())
    } else {
        Err(BridgeError::UnsupportedAst(format!(
            "operator token must be {type_name} {value:?}"
        )))
    }
}

fn ensure_not_is_masked(expr: &WirExpr, under_finite_mask: bool) -> Result<(), BridgeError> {
    match expr {
        WirExpr::Unary {
            op: UnaryOp::Not,
            expr,
        } => {
            if !under_finite_mask {
                return Err(BridgeError::InvalidNotContext);
            }
            ensure_not_is_masked(expr, false)
        }
        WirExpr::Binary {
            op: BinaryOp::BitAnd,
            lhs,
            rhs,
        } => {
            let bounds_rhs =
                under_finite_mask || matches!(lhs.as_ref(), WirExpr::Var(var) if var.0 == "mask");
            ensure_not_is_masked(lhs, false)?;
            ensure_not_is_masked(rhs, bounds_rhs)
        }
        WirExpr::Binary { lhs, rhs, .. } => {
            ensure_not_is_masked(lhs, false)?;
            ensure_not_is_masked(rhs, false)
        }
        WirExpr::ZExt { expr, .. } | WirExpr::Trunc { expr, .. } => {
            ensure_not_is_masked(expr, under_finite_mask)
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn node(rule_name: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule_name.to_string(),
            children,
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        }
    }

    fn token(value: &str, type_: TokenType, type_name: Option<&str>) -> Token {
        Token {
            type_,
            value: value.to_string(),
            line: 1,
            column: 1,
            type_name: type_name.map(str::to_string),
            flags: None,
            cv: None,
        }
    }

    const FIXTURE: &str = include_str!("../tests/fixtures/digital.ts");

    fn parsed_fixture() -> GrammarASTNode {
        parse_typescript(FIXTURE, TYPESCRIPT_VERSION).expect("fixture parses")
    }

    fn find_node_mut<'a>(
        node: &'a mut GrammarASTNode,
        rule: &str,
    ) -> Option<&'a mut GrammarASTNode> {
        if node.rule_name == rule {
            return Some(node);
        }
        for child in &mut node.children {
            if let ASTNodeOrToken::Node(child) = child {
                if let Some(found) = find_node_mut(child, rule) {
                    return Some(found);
                }
            }
        }
        None
    }

    #[test]
    fn error_variants_have_stable_nonempty_display_messages() {
        let errors = [
            BridgeError::SourceTooLarge {
                actual: 5,
                limit: 4,
            },
            BridgeError::InvalidUtf8,
            BridgeError::TooManyTokens {
                actual: 5,
                limit: 4,
            },
            BridgeError::Parse("bad syntax".into()),
            BridgeError::TooManyAstNodes {
                actual: 5,
                limit: 4,
            },
            BridgeError::UnsupportedAst("unsupported".into()),
            BridgeError::TypeCheck("type mismatch".into()),
            BridgeError::TooManyWirNodes { limit: 4 },
            BridgeError::InvalidNotContext,
            BridgeError::RestrictedProductionLineTerminator,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn ast_walkers_and_expected_child_helpers_handle_both_child_kinds() {
        let leaf = node("leaf", vec![]);
        let mixed = node(
            "mixed",
            vec![
                ASTNodeOrToken::Node(leaf.clone()),
                ASTNodeOrToken::Token(token(";", TokenType::Semicolon, None)),
            ],
        );
        assert_eq!(node_children(&mixed).len(), 1);
        assert_eq!(token_child(&mixed, 0), None);
        assert_eq!(token_child(&mixed, 1).unwrap().value, ";");
        assert_eq!(node_child(&mixed, 0).unwrap().rule_name, "leaf");
        assert_eq!(node_child(&mixed, 1), None);
        assert!(expect_token(&mixed, 1, ";").is_ok());
        assert!(expect_token(&mixed, 1, ",").is_err());
        assert!(expect_token(&mixed, 0, "leaf").is_err());
        assert!(expect_rule(&mixed, 0, "leaf").is_ok());
        assert!(expect_rule(&mixed, 0, "other").is_err());
        assert!(expect_rule(&mixed, 1, "leaf").is_err());
    }

    #[test]
    fn ast_budget_is_bounded_and_counts_nested_nodes() {
        let within_limit = node(
            "root",
            (0..MAX_AST_NODES - 1)
                .map(|_| ASTNodeOrToken::Node(node("child", vec![])))
                .collect(),
        );
        assert_eq!(
            count_ast_nodes(&within_limit, MAX_AST_NODES + 1),
            MAX_AST_NODES
        );
        assert!(ensure_ast_budget(&within_limit).is_ok());

        let children = (0..MAX_AST_NODES)
            .map(|_| ASTNodeOrToken::Node(node("child", vec![])))
            .collect();
        let oversized = node("root", children);
        assert_eq!(count_ast_nodes(&oversized, 3), 3);
        assert!(matches!(
            ensure_ast_budget(&oversized),
            Err(BridgeError::TooManyAstNodes { actual, limit: MAX_AST_NODES })
                if actual == MAX_AST_NODES + 1
        ));
        assert_eq!(count_ast_nodes(&node("empty", vec![]), 2), 1);
    }

    #[test]
    fn byte_offsets_are_unicode_scalar_based_and_reject_invalid_positions() {
        assert_eq!(source_offset("aλb\nxyz", 1, 3), Some(3));
        assert_eq!(source_offset("aλb\nxyz", 2, 2), Some(6));
        assert_eq!(source_offset("abc", 0, 1), None);
        assert_eq!(source_offset("abc", 1, 0), None);
        assert_eq!(source_offset("abc", 2, 1), None);
    }

    #[test]
    fn wir_node_budget_reports_the_first_overflow() {
        let mut budget = WirBudget::default();
        for _ in 0..MAX_WIR_EXPR_NODES {
            assert!(budget.take().is_ok());
        }
        assert_eq!(
            budget.take(),
            Err(BridgeError::TooManyWirNodes {
                limit: MAX_WIR_EXPR_NODES
            })
        );
    }

    #[test]
    fn helper_role_mapping_and_type_chain_refusal_are_explicit() {
        assert_eq!(HelperRole::from_index(0).name(), "bit_is_set");
        assert_eq!(HelperRole::from_index(1).name(), "set_bit");
        assert_eq!(HelperRole::from_index(2).name(), "clear_bit");
        let invalid_type = node(
            "type_expression",
            vec![ASTNodeOrToken::Node(node("not_conditional", vec![]))],
        );
        assert!(only_type(&invalid_type, "bigint").is_err());
        assert!(lower_program(&node("not_program", vec![]), "").is_err());
        assert!(lower_program(&node("program", vec![]), "").is_err());
    }

    #[test]
    fn ast_shape_mutations_fail_closed_at_each_declaration_boundary() {
        let mut ast = parsed_fixture();
        let source_element = find_node_mut(&mut ast, "source_element").unwrap();
        source_element.children.push(ASTNodeOrToken::Token(token(
            ";",
            TokenType::Semicolon,
            None,
        )));
        assert!(lower_program(&ast, FIXTURE).is_err());

        let mut ast = parsed_fixture();
        find_node_mut(&mut ast, "export_declaration")
            .unwrap()
            .children
            .pop();
        assert!(lower_program(&ast, FIXTURE).is_err());

        let mut ast = parsed_fixture();
        find_node_mut(&mut ast, "function_declaration")
            .unwrap()
            .children
            .pop();
        assert!(lower_program(&ast, FIXTURE).is_err());

        let mut ast = parsed_fixture();
        if let ASTNodeOrToken::Token(name) = &mut find_node_mut(&mut ast, "function_declaration")
            .unwrap()
            .children[1]
        {
            name.value = "renamed".into();
        }
        assert!(lower_program(&ast, FIXTURE).is_err());

        let mut ast = parsed_fixture();
        find_node_mut(&mut ast, "typed_parameter_list")
            .unwrap()
            .children
            .pop();
        assert!(lower_program(&ast, FIXTURE).is_err());

        let mut ast = parsed_fixture();
        find_node_mut(&mut ast, "function_body")
            .unwrap()
            .children
            .push(ASTNodeOrToken::Node(node("source_element", vec![])));
        assert!(lower_program(&ast, FIXTURE).is_err());

        let mut ast = parsed_fixture();
        find_node_mut(&mut ast, "return_statement")
            .unwrap()
            .children
            .pop();
        assert!(lower_program(&ast, FIXTURE).is_err());

        assert_eq!(first_token(&node("empty", vec![])), None);
    }

    #[test]
    fn type_parameter_and_expression_refusals_cover_ast_boundaries() {
        let predefined = node(
            "predefined_type",
            vec![ASTNodeOrToken::Token(token(
                "string",
                TokenType::Name,
                None,
            ))],
        );
        let mut type_node = predefined;
        for (parent, child) in [
            ("primary_type", "predefined_type"),
            ("array_type", "primary_type"),
            ("intersection_type", "array_type"),
            ("union_type", "intersection_type"),
            ("conditional_type", "union_type"),
            ("type_expression", "conditional_type"),
        ] {
            type_node = node(parent, vec![ASTNodeOrToken::Node(type_node)]);
            assert_eq!(type_node.rule_name, parent);
            let _ = child;
        }
        assert!(only_type(&type_node, "bigint").is_err());

        let parameter = node("wrong_parameter", vec![]);
        assert!(check_parameter(&parameter, "mask", "bigint").is_err());
        let literal_primary = node(
            "primary_expression",
            vec![ASTNodeOrToken::Token(token(
                "mask",
                TokenType::String,
                None,
            ))],
        );
        assert!(lower_primary(&literal_primary, &mut WirBudget::default()).is_err());
        assert!(lower_primary(
            &node("primary_expression", vec![]),
            &mut WirBudget::default()
        )
        .is_err());
        assert!(
            lower_expression(&node("unary_expression", vec![]), &mut WirBudget::default()).is_err()
        );
        assert!(simple_primary_name(&node("member_expression", vec![])).is_err());

        let tilde = node(
            "unary_expression",
            vec![ASTNodeOrToken::Token(token(
                "~",
                TokenType::Name,
                Some("TILDE"),
            ))],
        );
        assert!(lower_expression(&tilde, &mut WirBudget::default()).is_err());
    }

    #[test]
    fn input_errors_and_operator_context_refusals_are_distinct() {
        assert_eq!(
            lower_typescript_bytes(&[0xff]),
            Err(BridgeError::InvalidUtf8)
        );
        assert!(matches!(
            lower_typescript_bytes(b"export function {"),
            Err(BridgeError::Parse(_))
        ));

        let lhs = node("left", vec![]);
        let rhs = node("right", vec![]);
        let invalid_operator = node(
            "equality_expression",
            vec![
                ASTNodeOrToken::Node(lhs.clone()),
                ASTNodeOrToken::Token(token("&", TokenType::Name, Some("NOT_AMPERSAND"))),
                ASTNodeOrToken::Node(rhs.clone()),
            ],
        );
        assert!(lower_expression(&invalid_operator, &mut WirBudget::default()).is_err());

        let wrong_rule = node(
            "shift_expression",
            vec![
                ASTNodeOrToken::Node(lhs.clone()),
                ASTNodeOrToken::Token(token("&", TokenType::Name, Some("AMPERSAND"))),
                ASTNodeOrToken::Node(rhs.clone()),
            ],
        );
        assert!(lower_expression(&wrong_rule, &mut WirBudget::default()).is_err());

        let bad_not = node(
            "unary_expression",
            vec![
                ASTNodeOrToken::Token(token("~", TokenType::Name, Some("TILDE"))),
                ASTNodeOrToken::Token(token("mask", TokenType::Name, None)),
            ],
        );
        assert!(lower_expression(&bad_not, &mut WirBudget::default()).is_err());

        let mut nonzero = FIXTURE.replacen("!== 0n", "!== 1n", 1);
        assert!(matches!(
            lower_typescript_bytes(nonzero.as_bytes()),
            Err(BridgeError::UnsupportedAst(_))
        ));
        nonzero = FIXTURE.replacen("BigInt(slot)", "Number(slot)", 1);
        assert!(matches!(
            lower_typescript_bytes(nonzero.as_bytes()),
            Err(BridgeError::UnsupportedAst(_))
        ));
        nonzero = FIXTURE.replacen("BigInt(slot)", "BigInt(slot, slot)", 1);
        assert!(matches!(
            lower_typescript_bytes(nonzero.as_bytes()),
            Err(BridgeError::UnsupportedAst(_))
        ));
    }
}
