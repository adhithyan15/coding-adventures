//! # `GrammarASTNode` → `javascript_ast::Program` bridge (CLOC12.136)
//!
//! The generic `GrammarParser` produces a `GrammarASTNode` tree whose
//! nodes are labelled by *grammar rule name* (e.g. `"if_statement"`,
//! `"additive_expression"`) and whose children are a flat `Vec` of
//! either nested `GrammarASTNode`s or raw `Token`s — reflecting the
//! exact EBNF structure of the grammar rule that matched.
//!
//! This module converts that generic tree into the typed
//! `javascript_ast::Program` that every downstream consumer
//! (typechecker, optimization passes, emitter) expects.
//!
//! # Coverage
//!
//! **Phase 1 subset** — the 12 Phase 1 statement variants, 15 Phase 1
//! expression variants, and 2 Phase 1 declaration variants defined in
//! `javascript-ast v0.7.0`.  Any grammar rule outside Phase 1 (async
//! functions, generators, classes, destructuring, for-in/of,
//! try-catch, template literals, etc.) returns
//! `Err(BridgeError::UnsupportedSyntax)` so callers can decide whether
//! to degrade gracefully or propagate the error.
//!
//! # Children layout
//!
//! The grammar parser *flattens* every matched EBNF element into the
//! parent node's `children` list.  For a rule
//! `if_statement = "if" LPAREN expression RPAREN statement [ "else" statement ]`
//! the children are (for the with-else case):
//! ```text
//! [Token("if"), Token("("), Node("expression"), Token(")"),
//!  Node("statement"), Token("else"), Node("statement")]
//! ```
//! Alternation rules (`statement = block | if_statement | ...`) produce
//! the children of whichever alternative matched — typically a single
//! `Node` child with the alternative's rule name.
//!
//! Repetition `{ x }` appends each iteration's children in-place
//! (no wrapper node), so binary expression rules like
//! `additive = multiplicative { (PLUS|MINUS) multiplicative }` produce
//! `[Node, Token(op), Node, Token(op), Node, ...]`.
//!
//! # CV tracking
//!
//! v1: all produced nodes carry `cv: None`.  Per-node CV threading
//! (source-byte → IR → engine-clause provenance) is a follow-up
//! (CLOC12.137) that wires the lexer-level CVs into each AST node.

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use coding_adventures_javascript_ast::{
    declaration::{
        BindingTarget, Declaration, FunctionDeclaration, FunctionParam, VarKind,
        VariableDeclaration, VariableDeclarator,
    },
    expression::{
        ArrayExpression, AssignmentExpression, AssignmentOperator, AssignmentTarget,
        BigIntLiteral, BinaryExpression, BinaryOperator, BooleanLiteral, CallExpression,
        ConditionalExpression, Expression, Identifier, LogicalExpression, LogicalOperator,
        MemberExpression, NullLiteral, NumericLiteral, ObjectExpression, Property,
        PropertyKey, PropertyKind, StringLiteral, UnaryExpression, UnaryOperator,
        UndefinedLiteral,
    },
    statement::{
        BlockStatement, BreakStatement, ContinueStatement, EmptyStatement,
        ExpressionStatement, ForInit, ForStatement, IfStatement, LabeledStatement,
        ReturnStatement, Statement, SwitchCase, SwitchStatement, ThrowStatement,
        WhileStatement,
    },
    Program, ProgramItem, SourceType,
};
use coding_adventures_javascript_tokens::EsVersion;

// =========================================================================
// Public error type
// =========================================================================

/// Error returned when the bridge cannot convert a `GrammarASTNode` subtree
/// to a typed `javascript_ast` node.
#[derive(Debug, Clone)]
pub enum BridgeError {
    /// The grammar rule is valid JavaScript but is not yet covered by
    /// Phase 1 of the typed AST.  The optimization passes cannot handle
    /// it; callers should degrade gracefully (e.g. fall back to
    /// identity / WHITESPACE_ONLY output for SIMPLE/ADVANCED levels).
    UnsupportedSyntax {
        rule: String,
        location: String,
    },
    /// The node structure does not match the expected shape.  This
    /// indicates a mismatch between the grammar and the bridge — a bug.
    InternalError {
        msg: String,
        rule: String,
    },
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::UnsupportedSyntax { rule, location } => {
                write!(f, "UnsupportedSyntax: grammar rule '{rule}' at {location} is not yet covered by Phase 1 of the typed AST")
            }
            BridgeError::InternalError { msg, rule } => {
                write!(f, "InternalError in bridge for rule '{rule}': {msg}")
            }
        }
    }
}

impl std::error::Error for BridgeError {}

// =========================================================================
// Public entry point
// =========================================================================

/// Convert a `GrammarASTNode` (root rule: `"program"`) produced by the
/// grammar-driven JavaScript parser into a typed `javascript_ast::Program`.
///
/// Returns `Err(BridgeError::UnsupportedSyntax)` for any grammar rule
/// not yet covered by Phase 1 (async, generators, classes, for-in/of,
/// try-catch, destructuring, template literals, etc.).
///
/// # CV tracking
///
/// v1: all nodes carry `cv: None`.  Source-map-quality per-node CVs
/// land in CLOC12.137.
pub fn grammar_to_program(
    node: &GrammarASTNode,
    version: EsVersion,
) -> Result<Program, BridgeError> {
    if node.rule_name != "program" {
        return Err(BridgeError::InternalError {
            msg: format!("expected root rule 'program', got '{}'", node.rule_name),
            rule: node.rule_name.clone(),
        });
    }
    convert_program(node, version)
}

// =========================================================================
// Children helpers
// =========================================================================

/// All `ASTNodeOrToken::Node` children, in order.
fn node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// All Token values from the children (skips Node children).
fn token_vals(node: &GrammarASTNode) -> Vec<&str> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
            ASTNodeOrToken::Node(_) => None,
        })
        .collect()
}

/// True if the children list contains a Token with this exact value.
fn has_token(node: &GrammarASTNode, val: &str) -> bool {
    node.children.iter().any(|c| match c {
        ASTNodeOrToken::Token(t) => t.value == val,
        _ => false,
    })
}

/// If there is exactly one Node child, return it; else `None`.
fn sole_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    let nodes = node_children(node);
    if nodes.len() == 1 { Some(nodes[0]) } else { None }
}

/// Location string for error messages.
fn loc(node: &GrammarASTNode) -> String {
    match (node.start_line, node.start_column) {
        (Some(l), Some(c)) => format!("{l}:{c}"),
        (Some(l), None) => format!("{l}:?"),
        _ => "?:?".to_string(),
    }
}

fn unsupported(node: &GrammarASTNode) -> BridgeError {
    BridgeError::UnsupportedSyntax {
        rule: node.rule_name.clone(),
        location: loc(node),
    }
}

fn internal(node: &GrammarASTNode, msg: impl Into<String>) -> BridgeError {
    BridgeError::InternalError {
        msg: msg.into(),
        rule: node.rule_name.clone(),
    }
}

// =========================================================================
// Program
// =========================================================================

fn convert_program(node: &GrammarASTNode, version: EsVersion) -> Result<Program, BridgeError> {
    // program = [ HASHBANG ] { source_element }
    // Children: optional Token(#!) plus Node("source_element") per statement/decl.
    let mut body = Vec::new();
    for child in node_children(node) {
        let item = convert_source_element(child)?;
        body.push(item);
    }
    Ok(Program {
        cv: None,
        version,
        source_type: SourceType::Script,
        body,
    })
}

fn convert_source_element(node: &GrammarASTNode) -> Result<ProgramItem, BridgeError> {
    // source_element = import_declaration | export_declaration
    //                | function_declaration | generator_declaration
    //                | async_function_declaration | async_generator_declaration
    //                | decorated_class_declaration | class_declaration
    //                | statement ;
    // Alternation → exactly one child Node.
    let child = sole_node(node).ok_or_else(|| internal(node, "expected 1 child"))?;
    match child.rule_name.as_str() {
        "function_declaration" => {
            let decl = convert_function_declaration(child)?;
            Ok(ProgramItem::Declaration(Declaration::FunctionDeclaration(decl)))
        }
        "statement" => {
            let stmt = convert_statement(child)?;
            Ok(ProgramItem::Statement(stmt))
        }
        // variable_statement / lexical_declaration land inside statement
        _ => Err(unsupported(child)),
    }
}

// =========================================================================
// Statements
// =========================================================================

fn convert_statement(node: &GrammarASTNode) -> Result<Statement, BridgeError> {
    // statement = block | variable_statement | lexical_declaration
    //           | empty_statement | expression_statement | if_statement
    //           | while_statement | do_while_statement | for_statement
    //           | for_in_statement | for_of_statement | for_await_of_statement
    //           | continue_statement | break_statement | return_statement
    //           | with_statement | switch_statement | labelled_statement
    //           | try_statement | throw_statement | debugger_statement
    //           | using_declaration | await_using_declaration ;
    let child = sole_node(node).ok_or_else(|| internal(node, "expected 1 child in statement"))?;
    match child.rule_name.as_str() {
        "block" => convert_block_statement(child).map(Statement::block_statement),
        "variable_statement" => convert_variable_statement(child)
            .map(|v| Statement::Declaration(Declaration::VariableDeclaration(v))),
        "lexical_declaration" => convert_lexical_declaration(child)
            .map(|v| Statement::Declaration(Declaration::VariableDeclaration(v))),
        "empty_statement" => Ok(Statement::empty_statement(EmptyStatement { cv: None })),
        "expression_statement" => convert_expression_statement(child),
        "if_statement" => convert_if_statement(child).map(Statement::if_statement),
        "while_statement" => convert_while_statement(child).map(Statement::while_statement),
        "for_statement" => convert_for_statement(child).map(Statement::for_statement),
        "continue_statement" => convert_continue_statement(child).map(Statement::continue_statement),
        "break_statement" => convert_break_statement(child).map(Statement::break_statement),
        "return_statement" => convert_return_statement(child).map(Statement::return_statement),
        "switch_statement" => convert_switch_statement(child).map(Statement::switch_statement),
        "labelled_statement" => convert_labeled_statement(child).map(Statement::labeled_statement),
        "throw_statement" => convert_throw_statement(child).map(Statement::throw_statement),
        // Phase 2+ — not yet in the typed AST
        "do_while_statement" | "for_in_statement" | "for_of_statement"
        | "for_await_of_statement" | "try_statement" | "with_statement"
        | "debugger_statement" | "using_declaration" | "await_using_declaration" => {
            Err(unsupported(child))
        }
        other => Err(BridgeError::InternalError {
            msg: format!("unknown statement child rule '{other}'"),
            rule: node.rule_name.clone(),
        }),
    }
}

// -------------------------------------------------------------------------
// block
// -------------------------------------------------------------------------

fn convert_block_statement(node: &GrammarASTNode) -> Result<BlockStatement, BridgeError> {
    // block = LBRACE { statement } RBRACE ;
    // All Node children are statement nodes.
    let stmts: Result<Vec<Statement>, _> = node_children(node)
        .into_iter()
        .map(convert_statement)
        .collect();
    Ok(BlockStatement { cv: None, body: stmts? })
}

// -------------------------------------------------------------------------
// if_statement
// -------------------------------------------------------------------------

fn convert_if_statement(node: &GrammarASTNode) -> Result<IfStatement, BridgeError> {
    // if_statement = "if" LPAREN expression RPAREN statement [ "else" statement ]
    // Node children: [expression, statement] or [expression, statement, statement]
    let nodes = node_children(node);
    if nodes.len() < 2 {
        return Err(internal(node, "if_statement needs ≥2 node children"));
    }
    let test = convert_expression(nodes[0])?;
    let consequent = convert_statement(nodes[1])?;
    let alternate = if nodes.len() >= 3 {
        Some(Box::new(convert_statement(nodes[2])?))
    } else {
        None
    };
    Ok(IfStatement {
        cv: None,
        test,
        consequent: Box::new(consequent),
        alternate,
    })
}

// -------------------------------------------------------------------------
// while_statement
// -------------------------------------------------------------------------

fn convert_while_statement(node: &GrammarASTNode) -> Result<WhileStatement, BridgeError> {
    // while_statement = "while" LPAREN expression RPAREN statement
    // Node children: [expression, statement]
    let nodes = node_children(node);
    if nodes.len() < 2 {
        return Err(internal(node, "while_statement needs 2 node children"));
    }
    Ok(WhileStatement {
        cv: None,
        test: convert_expression(nodes[0])?,
        body: Box::new(convert_statement(nodes[1])?),
    })
}

// -------------------------------------------------------------------------
// for_statement
// -------------------------------------------------------------------------

fn convert_for_statement(node: &GrammarASTNode) -> Result<ForStatement, BridgeError> {
    // for_statement = "for" LPAREN
    //   [ ( "var" variable_declaration_list | expression ) ]
    //   SEMICOLON
    //   [ expression ]
    //   SEMICOLON
    //   [ expression ]
    //   RPAREN statement ;
    //
    // Walk children left-to-right, use SEMICOLON tokens as delimiters.
    // Node children: init? test? update? body
    // We'll scan by looking for SEMICOLON tokens and RPAREN.

    let mut init: Option<ForInit> = None;
    let mut test: Option<Expression> = None;
    let mut update: Option<Expression> = None;
    let mut body_node: Option<&GrammarASTNode> = None;

    // Phase: 0=before first ;, 1=between ; and ;, 2=between ; and ),
    // 3=after ) (body)
    let mut phase = 0usize;
    let mut phase_nodes: [Vec<&GrammarASTNode>; 4] = [vec![], vec![], vec![], vec![]];

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if t.value == ";" => {
                if phase < 2 { phase += 1; }
            }
            ASTNodeOrToken::Token(t) if t.value == ")" => {
                if phase == 2 { phase = 3; }
            }
            ASTNodeOrToken::Node(n) => {
                phase_nodes[phase.min(3)].push(n);
            }
            ASTNodeOrToken::Token(_) => {}
        }
    }

    // Phase 0: init (variable_declaration_list or expression)
    if let Some(&n) = phase_nodes[0].first() {
        match n.rule_name.as_str() {
            "variable_declaration_list" => {
                let decl = convert_var_decl_list(n, VarKind::Var)?;
                init = Some(ForInit::VariableDeclaration(decl));
            }
            _ => {
                init = Some(ForInit::Expression(convert_expression(n)?));
            }
        }
    }

    // Phase 1: test
    if let Some(&n) = phase_nodes[1].first() {
        test = Some(convert_expression(n)?);
    }

    // Phase 2: update
    if let Some(&n) = phase_nodes[2].first() {
        update = Some(convert_expression(n)?);
    }

    // Phase 3: body
    let body_n = phase_nodes[3].first().copied().or_else(|| {
        // Fallback: last node overall is the body.
        node_children(node).into_iter().last()
    });
    if let Some(n) = body_n {
        body_node = Some(n);
    }

    let body = body_node
        .ok_or_else(|| internal(node, "for_statement: missing body"))?;

    Ok(ForStatement {
        cv: None,
        init,
        test,
        update,
        body: Box::new(convert_statement(body)?),
    })
}

// -------------------------------------------------------------------------
// continue / break / return / throw / switch / labeled
// -------------------------------------------------------------------------

fn convert_continue_statement(node: &GrammarASTNode) -> Result<ContinueStatement, BridgeError> {
    // continue_statement = "continue" [ NAME ] SEMICOLON ;
    // Token children: ["continue", optional_name, ";"]
    // We need the NAME token if present.
    let label = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t)
            if t.value != "continue" && t.value != ";" =>
        {
            Some(Identifier { cv: None, name: t.value.clone() })
        }
        _ => None,
    });
    Ok(ContinueStatement { cv: None, label })
}

fn convert_break_statement(node: &GrammarASTNode) -> Result<BreakStatement, BridgeError> {
    // break_statement = "break" [ NAME ] SEMICOLON ;
    let label = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t)
            if t.value != "break" && t.value != ";" =>
        {
            Some(Identifier { cv: None, name: t.value.clone() })
        }
        _ => None,
    });
    Ok(BreakStatement { cv: None, label })
}

fn convert_return_statement(node: &GrammarASTNode) -> Result<ReturnStatement, BridgeError> {
    // return_statement = "return" [ expression ] SEMICOLON ;
    let expr = match node_children(node).first() {
        Some(&n) => Some(convert_expression(n)?),
        None => None,
    };
    Ok(ReturnStatement { cv: None, argument: expr })
}

fn convert_throw_statement(node: &GrammarASTNode) -> Result<ThrowStatement, BridgeError> {
    // throw_statement = "throw" expression SEMICOLON ;
    let n = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "throw_statement: missing expression"))?;
    Ok(ThrowStatement { cv: None, argument: convert_expression(n)? })
}

fn convert_switch_statement(node: &GrammarASTNode) -> Result<SwitchStatement, BridgeError> {
    // switch_statement = "switch" LPAREN expression RPAREN
    //   LBRACE { case_clause | default_clause } RBRACE ;
    let nodes = node_children(node);
    let discriminant = nodes
        .first()
        .ok_or_else(|| internal(node, "switch_statement: missing discriminant"))?;
    let discriminant = convert_expression(discriminant)?;

    let mut cases = Vec::new();
    for n in nodes.iter().skip(1) {
        match n.rule_name.as_str() {
            "case_clause" => cases.push(convert_case_clause(n)?),
            "default_clause" => cases.push(convert_default_clause(n)?),
            _ => {}
        }
    }
    Ok(SwitchStatement { cv: None, discriminant, cases })
}

fn convert_case_clause(node: &GrammarASTNode) -> Result<SwitchCase, BridgeError> {
    // case_clause = "case" expression COLON { statement }
    let nodes = node_children(node);
    let test_n = nodes
        .first()
        .ok_or_else(|| internal(node, "case_clause: missing test expression"))?;
    let test = Some(convert_expression(test_n)?);
    let consequent: Result<Vec<Statement>, _> =
        nodes.iter().skip(1).map(|n| convert_statement(n)).collect();
    Ok(SwitchCase { cv: None, test, consequent: consequent? })
}

fn convert_default_clause(node: &GrammarASTNode) -> Result<SwitchCase, BridgeError> {
    // default_clause = "default" COLON { statement }
    let consequent: Result<Vec<Statement>, _> =
        node_children(node).into_iter().map(|n| convert_statement(n)).collect();
    Ok(SwitchCase { cv: None, test: None, consequent: consequent? })
}

fn convert_labeled_statement(node: &GrammarASTNode) -> Result<LabeledStatement, BridgeError> {
    // labelled_statement = NAME COLON statement ;
    // Token children include the name; one Node child is the statement.
    let label_val = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.value != ":" => Some(t.value.clone()),
        _ => None,
    });
    let label_name = label_val
        .ok_or_else(|| internal(node, "labelled_statement: missing label name"))?;
    let body_n = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "labelled_statement: missing body statement"))?;
    Ok(LabeledStatement {
        cv: None,
        label: Identifier { cv: None, name: label_name },
        body: Box::new(convert_statement(body_n)?),
    })
}

// -------------------------------------------------------------------------
// expression_statement
// -------------------------------------------------------------------------

fn convert_expression_statement(node: &GrammarASTNode) -> Result<Statement, BridgeError> {
    // expression_statement = expression SEMICOLON ;
    let n = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "expression_statement: missing expression"))?;
    let expr = convert_expression(n)?;
    Ok(Statement::expression_statement(ExpressionStatement { cv: None, expression: expr }))
}

// =========================================================================
// Variable declarations
// =========================================================================

fn convert_variable_statement(node: &GrammarASTNode) -> Result<VariableDeclaration, BridgeError> {
    // variable_statement = "var" variable_declaration_list SEMICOLON ;
    // One Node child: variable_declaration_list
    let list_n = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "variable_statement: missing declaration list"))?;
    convert_var_decl_list(list_n, VarKind::Var)
}

fn convert_var_decl_list(
    node: &GrammarASTNode,
    kind: VarKind,
) -> Result<VariableDeclaration, BridgeError> {
    // variable_declaration_list = variable_declaration { COMMA variable_declaration }
    // All Node children are variable_declaration.
    let declarators: Result<Vec<VariableDeclarator>, _> = node_children(node)
        .into_iter()
        .map(|n| convert_variable_declarator(n))
        .collect();
    Ok(VariableDeclaration { cv: None, kind, declarations: declarators? })
}

fn convert_variable_declarator(node: &GrammarASTNode) -> Result<VariableDeclarator, BridgeError> {
    // variable_declaration = ( NAME | binding_pattern ) [ EQUALS assignment_expression ]
    // OR: lexical_binding = ( NAME | binding_pattern ) [ EQUALS assignment_expression ]
    //
    // In both cases: first token or node is the binding name; optional
    // second node is the initializer expression.

    // Find the NAME token (the binding identifier).
    let id_name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.value != "=" => Some(t.value.clone()),
        _ => None,
    });
    let id_name = id_name.ok_or_else(|| internal(node, "variable declarator: missing name"))?;

    // Check for binding_pattern (destructuring) — not in Phase 1.
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "binding_pattern" {
                return Err(unsupported(n));
            }
        }
    }

    // Find the initializer: the expression node child.
    let init = node_children(node).into_iter().next();
    let init_expr = match init {
        Some(n) if n.rule_name == "assignment_expression" || n.rule_name.contains("expression") => {
            Some(convert_expression(n)?)
        }
        _ => None,
    };

    Ok(VariableDeclarator {
        cv: None,
        id: BindingTarget::Identifier(Identifier { cv: None, name: id_name }),
        init: init_expr,
    })
}

fn convert_lexical_declaration(node: &GrammarASTNode) -> Result<VariableDeclaration, BridgeError> {
    // lexical_declaration = ( "let" | "const" ) binding_list SEMICOLON ;
    // binding_list = lexical_binding { COMMA lexical_binding }
    let tok_vals = token_vals(node);
    let kind = if tok_vals.contains(&"let") {
        VarKind::Let
    } else {
        VarKind::Const
    };
    // Node child is binding_list
    let list_n = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "lexical_declaration: missing binding_list"))?;

    // binding_list node children are lexical_binding nodes.
    let declarators: Result<Vec<VariableDeclarator>, _> = node_children(list_n)
        .into_iter()
        .map(|n| convert_variable_declarator(n))
        .collect();
    Ok(VariableDeclaration { cv: None, kind, declarations: declarators? })
}

// =========================================================================
// Function declaration
// =========================================================================

fn convert_function_declaration(node: &GrammarASTNode) -> Result<FunctionDeclaration, BridgeError> {
    // function_declaration = "function" NAME LPAREN [ formal_parameters ] RPAREN
    //                        LBRACE function_body RBRACE ;
    // Token children include "function", NAME, "(", ")", "{", "}"
    // Node children: optional formal_parameters, then function_body

    // Extract function name from token children (skip "function").
    let name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t)
            if t.value != "function" && t.value != "(" && t.value != ")" && t.value != "{" && t.value != "}" =>
        {
            Some(t.value.clone())
        }
        _ => None,
    });
    let name = name.ok_or_else(|| internal(node, "function_declaration: missing name"))?;

    let nodes = node_children(node);
    let mut params = Vec::new();
    let mut body_node: Option<&GrammarASTNode> = None;

    for n in &nodes {
        match n.rule_name.as_str() {
            "formal_parameters" => {
                params = convert_formal_parameters(n)?;
            }
            "function_body" => {
                body_node = Some(n);
            }
            _ => {}
        }
    }

    let body = match body_node {
        Some(n) => convert_function_body(n)?,
        None => BlockStatement { cv: None, body: vec![] },
    };

    Ok(FunctionDeclaration {
        cv: None,
        id: Identifier { cv: None, name },
        params,
        body,
        generator: false,
        is_async: false,
    })
}

fn convert_formal_parameters(node: &GrammarASTNode) -> Result<Vec<FunctionParam>, BridgeError> {
    // formal_parameters = formal_parameter { COMMA formal_parameter } [ COMMA ]
    // formal_parameter = ( NAME | binding_pattern ) [ EQUALS assignment_expression ]
    //                  | ELLIPSIS ( NAME | binding_pattern )
    let params: Result<Vec<FunctionParam>, _> = node_children(node)
        .into_iter()
        .map(|n| convert_formal_parameter(n))
        .collect();
    params
}

fn convert_formal_parameter(node: &GrammarASTNode) -> Result<FunctionParam, BridgeError> {
    // formal_parameter = ( NAME | binding_pattern ) [ EQUALS assignment_expression ]
    //                  | ELLIPSIS ( NAME | binding_pattern )
    // In Phase 1: only simple NAME identifiers.
    if has_token(node, "...") {
        return Err(unsupported(node)); // rest params are Phase 3
    }
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "binding_pattern" {
                return Err(unsupported(n));
            }
        }
    }
    // Has default? Not Phase 1.
    if has_token(node, "=") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "formal_parameter_default".to_string(),
            location: loc(node),
        });
    }
    let name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) => Some(t.value.clone()),
        _ => None,
    });
    let name = name.ok_or_else(|| internal(node, "formal_parameter: missing name"))?;
    Ok(FunctionParam::Identifier(Identifier { cv: None, name }))
}

fn convert_function_body(node: &GrammarASTNode) -> Result<BlockStatement, BridgeError> {
    // function_body = { source_element }
    let stmts: Result<Vec<Statement>, _> = node_children(node)
        .into_iter()
        .map(|n| {
            // source_element children are statements or declarations.
            let item = convert_source_element(n)?;
            Ok(match item {
                ProgramItem::Statement(s) => s,
                ProgramItem::Declaration(d) => Statement::Declaration(d),
            })
        })
        .collect();
    Ok(BlockStatement { cv: None, body: stmts? })
}

// =========================================================================
// Expressions — dispatch
// =========================================================================

fn convert_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    match node.rule_name.as_str() {
        // Top-level expression rule: comma-separated sequence.
        "expression" => convert_expression_rule(node),

        // Assignment
        "assignment_expression" => convert_assignment_expression(node),

        // Conditional and below
        "conditional_expression" => convert_conditional_expression(node),

        // Short-circuit logical
        "nullish_coalescing_expression" => convert_left_fold_logical(node, "??"),
        "logical_or_expression" => convert_left_fold_logical(node, "||"),
        "logical_and_expression" => convert_left_fold_logical(node, "&&"),

        // Bitwise
        "bitwise_or_expression" => convert_left_fold_binary(node, &["|"]),
        "bitwise_xor_expression" => convert_left_fold_binary(node, &["^"]),
        "bitwise_and_expression" => convert_left_fold_binary(node, &["&"]),

        // Equality / relational / shift / additive / multiplicative / exponentiation
        "equality_expression" => convert_left_fold_binary(node, &["==", "!=", "===", "!=="]),
        "relational_expression" => {
            convert_left_fold_binary(node, &["<", ">", "<=", ">=", "in", "instanceof"])
        }
        "shift_expression" => convert_left_fold_binary(node, &["<<", ">>", ">>>"]),
        "additive_expression" => convert_left_fold_binary(node, &["+", "-"]),
        "multiplicative_expression" => convert_left_fold_binary(node, &["*", "/", "%"]),
        "exponentiation_expression" => convert_right_fold_binary(node, "**"),

        // Unary / postfix
        "unary_expression" => convert_unary_expression(node),
        "postfix_expression" => convert_postfix_expression(node),

        // LHS / call / member / primary
        "left_hand_side_expression" => convert_lhs_expression(node),
        "call_expression" => convert_call_expression(node),
        // optional_chain_expression is the main suffix-chain rule in the grammar —
        // it handles simple member_expression pass-through, dot access, bracket
        // access, AND function calls in addition to true optional-chain (?.); only
        // the ?. variants are Phase 2.
        "optional_chain_expression" => convert_optional_chain_expression(node),
        "new_expression" => convert_new_expression(node),
        "member_expression" => convert_member_expression(node),
        "primary_expression" => convert_primary_expression(node),

        // Literals
        "array_literal" => convert_array_literal(node),
        "object_literal" => convert_object_literal(node),

        // ES2015+ unsupported in Phase 1
        "arrow_function" | "async_arrow_function" | "yield_expression"
        | "await_expression" | "generator_expression" | "async_function_expression"
        | "async_generator_expression" | "class_expression"
        | "template_literal" | "tagged_template_expression"
        | "new_target_expression" | "import_meta_expression" => Err(unsupported(node)),

        other => Err(BridgeError::InternalError {
            msg: format!("unknown expression rule '{other}'"),
            rule: node.rule_name.clone(),
        }),
    }
}

// -------------------------------------------------------------------------
// expression rule (comma-sequence)
// -------------------------------------------------------------------------

fn convert_expression_rule(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // expression = assignment_expression { COMMA assignment_expression }
    // Phase 1: only single-expression form. Multi-expression (sequence) is Phase 5.
    let nodes = node_children(node);
    if nodes.len() == 1 {
        convert_expression(nodes[0])
    } else if nodes.is_empty() {
        Err(internal(node, "expression: no children"))
    } else {
        // Multiple expressions separated by comma = SequenceExpression (not in Phase 1).
        Err(BridgeError::UnsupportedSyntax {
            rule: "SequenceExpression".to_string(),
            location: loc(node),
        })
    }
}

// -------------------------------------------------------------------------
// assignment_expression
// -------------------------------------------------------------------------

fn convert_assignment_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // assignment_expression = conditional_expression
    //                       | left_hand_side_expression assignment_operator assignment_expression
    let nodes = node_children(node);
    if nodes.len() == 1 {
        // Pass-through to conditional_expression.
        return convert_expression(nodes[0]);
    }
    if nodes.len() == 3 {
        // nodes[0] = lhs, nodes[1] = assignment_operator, nodes[2] = rhs
        let lhs = convert_expression(nodes[0])?;
        let op_str = token_vals(nodes[1]).into_iter().next().unwrap_or("=");
        let op = parse_assignment_op(op_str).ok_or_else(|| {
            internal(node, format!("unknown assignment operator '{op_str}'"))
        })?;
        let rhs = convert_expression(nodes[2])?;
        let target = expr_to_assignment_target(lhs, node)?;
        return Ok(Expression::AssignmentExpression(AssignmentExpression {
            cv: None,
            operator: op,
            left: target,
            right: Box::new(rhs),
        }));
    }
    Err(internal(node, format!("assignment_expression: unexpected {} node children", nodes.len())))
}

fn expr_to_assignment_target(
    expr: Expression,
    ctx: &GrammarASTNode,
) -> Result<AssignmentTarget, BridgeError> {
    match expr {
        Expression::Identifier(id) => Ok(AssignmentTarget::Identifier(id)),
        Expression::MemberExpression(m) => {
            Ok(AssignmentTarget::MemberExpression(Box::new(m)))
        }
        _ => Err(BridgeError::UnsupportedSyntax {
            rule: "DestructuringAssignmentTarget".to_string(),
            location: loc(ctx),
        }),
    }
}

fn parse_assignment_op(s: &str) -> Option<AssignmentOperator> {
    match s {
        "=" => Some(AssignmentOperator::Eq),
        "+=" => Some(AssignmentOperator::AddEq),
        "-=" => Some(AssignmentOperator::SubEq),
        "*=" => Some(AssignmentOperator::MulEq),
        "/=" => Some(AssignmentOperator::DivEq),
        "%=" => Some(AssignmentOperator::ModEq),
        "**=" => Some(AssignmentOperator::ExpEq),
        "<<=" => Some(AssignmentOperator::LeftShiftEq),
        ">>=" => Some(AssignmentOperator::RightShiftEq),
        ">>>=" => Some(AssignmentOperator::UnsignedRightShiftEq),
        "|=" => Some(AssignmentOperator::BitOrEq),
        "^=" => Some(AssignmentOperator::BitXorEq),
        "&=" => Some(AssignmentOperator::BitAndEq),
        _ => None,
    }
}

// -------------------------------------------------------------------------
// conditional_expression
// -------------------------------------------------------------------------

fn convert_conditional_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // conditional_expression = nullish_coalescing_expression
    //                        | nullish_coalescing_expression "?" assignment_expression ":" assignment_expression
    let nodes = node_children(node);
    if nodes.len() == 1 {
        return convert_expression(nodes[0]);
    }
    if nodes.len() == 3 {
        // test ? consequent : alternate
        let test = convert_expression(nodes[0])?;
        let consequent = convert_expression(nodes[1])?;
        let alternate = convert_expression(nodes[2])?;
        return Ok(Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        }));
    }
    Err(internal(node, format!("conditional_expression: {} node children", nodes.len())))
}

// -------------------------------------------------------------------------
// Left-fold binary: A { op A } → BinaryExpression / LogicalExpression
// -------------------------------------------------------------------------

/// Left-fold for logical operators (&&, ||, ??)
fn convert_left_fold_logical(node: &GrammarASTNode, op_tok: &str) -> Result<Expression, BridgeError> {
    let nodes = node_children(node);
    if nodes.len() == 1 {
        return convert_expression(nodes[0]);
    }
    // Multiple children: fold left. Children interleave Node, Token(op), Node...
    let mut left = convert_expression(nodes[0])?;
    let op = parse_logical_op(op_tok).ok_or_else(|| {
        internal(node, format!("unknown logical op '{op_tok}'"))
    })?;
    for right_n in nodes.iter().skip(1) {
        let right = convert_expression(right_n)?;
        left = Expression::LogicalExpression(LogicalExpression {
            cv: None,
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
        });
    }
    Ok(left)
}

fn parse_logical_op(s: &str) -> Option<LogicalOperator> {
    match s {
        "&&" => Some(LogicalOperator::And),
        "||" => Some(LogicalOperator::Or),
        "??" => Some(LogicalOperator::NullishCoalescing),
        _ => None,
    }
}

/// Left-fold for binary operators (A { op A } form).
/// `allowed_ops` lists the expected operator tokens for this rule.
fn convert_left_fold_binary(
    node: &GrammarASTNode,
    allowed_ops: &[&str],
) -> Result<Expression, BridgeError> {
    let nodes = node_children(node);
    if nodes.len() == 1 {
        return convert_expression(nodes[0]);
    }
    // Walk the flat children list: Node, Token(op), Node, Token(op), Node...
    // We reconstruct the operator-node pairs from the interleaved children.
    let mut left_expr = convert_expression(nodes[0])?;
    let mut op_iter = node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Token(t) if allowed_ops.contains(&t.value.as_str()) => {
            Some(t.value.as_str())
        }
        _ => None,
    });
    for right_n in nodes.iter().skip(1) {
        let op_str = op_iter.next().ok_or_else(|| {
            internal(node, "convert_left_fold_binary: operator count < node count")
        })?;
        let op = parse_binary_op(op_str).ok_or_else(|| {
            internal(node, format!("unknown binary op '{op_str}'"))
        })?;
        let right = convert_expression(right_n)?;
        left_expr = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: op,
            left: Box::new(left_expr),
            right: Box::new(right),
        });
    }
    Ok(left_expr)
}

/// Right-fold for exponentiation: A ** A is right-associative.
fn convert_right_fold_binary(node: &GrammarASTNode, op_str: &str) -> Result<Expression, BridgeError> {
    let nodes = node_children(node);
    if nodes.len() == 1 {
        return convert_expression(nodes[0]);
    }
    if nodes.len() == 2 {
        let op = parse_binary_op(op_str).ok_or_else(|| {
            internal(node, format!("unknown binary op '{op_str}'"))
        })?;
        let left = convert_expression(nodes[0])?;
        let right = convert_expression(nodes[1])?;
        return Ok(Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
        }));
    }
    Err(internal(node, format!("exponentiation_expression: {} nodes", nodes.len())))
}

fn parse_binary_op(s: &str) -> Option<BinaryOperator> {
    match s {
        "==" => Some(BinaryOperator::Eq),
        "!=" => Some(BinaryOperator::NotEq),
        "===" => Some(BinaryOperator::StrictEq),
        "!==" => Some(BinaryOperator::StrictNotEq),
        "<" => Some(BinaryOperator::Lt),
        "<=" => Some(BinaryOperator::LtEq),
        ">" => Some(BinaryOperator::Gt),
        ">=" => Some(BinaryOperator::GtEq),
        "<<" => Some(BinaryOperator::LeftShift),
        ">>" => Some(BinaryOperator::RightShift),
        ">>>" => Some(BinaryOperator::UnsignedRightShift),
        "+" => Some(BinaryOperator::Add),
        "-" => Some(BinaryOperator::Sub),
        "*" => Some(BinaryOperator::Mul),
        "/" => Some(BinaryOperator::Div),
        "%" => Some(BinaryOperator::Mod),
        "**" => Some(BinaryOperator::Exp),
        "|" => Some(BinaryOperator::BitOr),
        "^" => Some(BinaryOperator::BitXor),
        "&" => Some(BinaryOperator::BitAnd),
        "in" => Some(BinaryOperator::In),
        "instanceof" => Some(BinaryOperator::InstanceOf),
        _ => None,
    }
}

// -------------------------------------------------------------------------
// unary_expression
// -------------------------------------------------------------------------

fn convert_unary_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // unary_expression = postfix_expression
    //                  | "delete" | "void" | "typeof" | PLUS | MINUS | TILDE | BANG
    //                    unary_expression
    let nodes = node_children(node);
    if nodes.len() == 1 {
        return convert_expression(nodes[0]); // pass-through
    }
    // Has a prefix operator token.
    let op_tok = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
        _ => None,
    });
    let op_str = op_tok.ok_or_else(|| internal(node, "unary_expression: missing operator token"))?;
    let op = match op_str {
        "-" => UnaryOperator::Negate,
        "+" => UnaryOperator::Plus,
        "!" => UnaryOperator::Not,
        "~" => UnaryOperator::BitNot,
        "typeof" => UnaryOperator::TypeOf,
        "void" => UnaryOperator::Void,
        "delete" => UnaryOperator::Delete,
        _ => return Err(internal(node, format!("unknown unary op '{op_str}'"))),
    };
    let arg_n = nodes
        .first()
        .ok_or_else(|| internal(node, "unary_expression: missing argument"))?;
    Ok(Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: op,
        prefix: true,
        argument: Box::new(convert_expression(arg_n)?),
    }))
}

// -------------------------------------------------------------------------
// postfix_expression
// -------------------------------------------------------------------------

fn convert_postfix_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // postfix_expression = left_hand_side_expression [ PLUS_PLUS | MINUS_MINUS ]
    let nodes = node_children(node);
    let has_postfix = has_token(node, "++") || has_token(node, "--");
    if has_postfix {
        // UpdateExpression — Phase 2, not in Phase 1 typed AST.
        return Err(BridgeError::UnsupportedSyntax {
            rule: "UpdateExpression".to_string(),
            location: loc(node),
        });
    }
    if nodes.len() == 1 {
        return convert_expression(nodes[0]);
    }
    Err(internal(node, "postfix_expression: unexpected shape"))
}

// -------------------------------------------------------------------------
// left_hand_side_expression
// -------------------------------------------------------------------------

fn convert_lhs_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // left_hand_side_expression = call_expression | optional_chain_expression | member_expression
    let child = sole_node(node).ok_or_else(|| internal(node, "lhs_expression: expected 1 child"))?;
    convert_expression(child)
}

// -------------------------------------------------------------------------
// optional_chain_expression
// -------------------------------------------------------------------------

/// Convert `optional_chain_expression` — the grammar rule that wraps ALL
/// suffix operations on a `member_expression` base:
///
/// ```text
/// optional_chain_expression = member_expression
///   { OPTIONAL_CHAIN NAME | OPTIONAL_CHAIN ... | DOT NAME | LBRACKET expr RBRACKET | arguments | template_literal }
/// ```
///
/// Phase 1 handles: simple pass-through (no suffix), dot-access, bracket-access,
/// and function-call suffixes. OPTIONAL_CHAIN (`?.`) suffixes are Phase 2.
fn convert_optional_chain_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // If there's a ?. token anywhere → Phase 2.
    if has_token(node, "?.") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "OptionalChainExpression".to_string(),
            location: loc(node),
        });
    }

    let nodes = node_children(node);
    if nodes.is_empty() {
        return Err(internal(node, "optional_chain_expression: no children"));
    }

    // Base: the first node child is always the member_expression.
    let mut base = convert_expression(nodes[0])?;

    // Walk any additional suffix operations in the children after the base.
    // Children layout after the base member_expression:
    //   DOT NAME → dot access
    //   LBRACKET Node(expression) RBRACKET → computed access
    //   Node(arguments) → function call
    //   Node(template_literal) → tagged template (Phase 2)
    //
    // We iterate the raw children list to find the suffixes.
    let mut i = 0;
    let children = &node.children;
    // Skip past the first Node (the base member_expression).
    while i < children.len() {
        if matches!(&children[i], ASTNodeOrToken::Node(_)) {
            i += 1;
            break;
        }
        i += 1;
    }

    // Now process remaining children as suffix groups.
    while i < children.len() {
        match &children[i] {
            ASTNodeOrToken::Token(t) if t.value == "." => {
                // DOT NAME — dot property access.
                i += 1;
                let prop_name = match children.get(i) {
                    Some(ASTNodeOrToken::Token(t)) => t.value.clone(),
                    _ => return Err(internal(node, "optional_chain_expression: missing property name after .")),
                };
                i += 1;
                base = Expression::MemberExpression(MemberExpression {
                    cv: None,
                    object: Box::new(base),
                    property: Box::new(Expression::Identifier(Identifier { cv: None, name: prop_name })),
                    computed: false,
                });
            }
            ASTNodeOrToken::Token(t) if t.value == "[" => {
                // LBRACKET expression RBRACKET — computed access.
                i += 1;
                // Find the Node child (the key expression).
                while i < children.len() {
                    if let ASTNodeOrToken::Node(key_n) = &children[i] {
                        let key = convert_expression(key_n)?;
                        base = Expression::MemberExpression(MemberExpression {
                            cv: None,
                            object: Box::new(base),
                            property: Box::new(key),
                            computed: true,
                        });
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                // Skip the RBRACKET.
                if i < children.len() {
                    if let ASTNodeOrToken::Token(t) = &children[i] {
                        if t.value == "]" { i += 1; }
                    }
                }
            }
            ASTNodeOrToken::Node(arg_n) if arg_n.rule_name == "arguments" => {
                // Function call via arguments node.
                let args = convert_arguments(arg_n)?;
                base = Expression::CallExpression(CallExpression {
                    cv: None,
                    callee: Box::new(base),
                    arguments: args,
                });
                i += 1;
            }
            ASTNodeOrToken::Node(n) if n.rule_name == "template_literal" || n.rule_name == "argument_list" => {
                // Tagged template → Phase 2.
                return Err(BridgeError::UnsupportedSyntax {
                    rule: n.rule_name.clone(),
                    location: loc(n),
                });
            }
            _ => {
                i += 1; // Skip unknown tokens (e.g. closing brackets already consumed).
            }
        }
    }

    Ok(base)
}

// -------------------------------------------------------------------------
// new_expression
// -------------------------------------------------------------------------

fn convert_new_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // new_expression = member_expression | "new" new_expression
    // Simple member_expression pass-through (no "new" keyword).
    if !has_token(node, "new") {
        let child = sole_node(node)
            .ok_or_else(|| internal(node, "new_expression: expected 1 child"))?;
        return convert_expression(child);
    }
    // "new X" — NewExpression, Phase 2.
    Err(BridgeError::UnsupportedSyntax {
        rule: "NewExpression".to_string(),
        location: loc(node),
    })
}

// -------------------------------------------------------------------------
// call_expression
// -------------------------------------------------------------------------

fn convert_call_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // call_expression = member_expression arguments
    //                 | call_expression arguments
    //                 | call_expression LBRACKET expression RBRACKET
    //                 | call_expression DOT NAME
    //                 | call_expression OPTIONAL_CHAINING ...
    //
    // The grammar parser handles left-recursion by breaking on the
    // first alternative. For simple calls `f(x)`:
    //   children: [Node("member_expression"), Node("arguments")]
    // For chained calls `f(x)(y)` the parser's left-recursion guard
    // means it will only capture the innermost call at this version.
    // More complex patterns are handled below by walking the child types.

    let nodes = node_children(node);
    if nodes.is_empty() {
        return Err(internal(node, "call_expression: no children"));
    }

    let last = nodes.last().unwrap();
    match last.rule_name.as_str() {
        "arguments" => {
            // f(args) — callee is everything before arguments
            let callee = if nodes.len() == 2 {
                convert_expression(nodes[0])?
            } else {
                // Recursive: re-interpret all but last as call_expression
                convert_expression(nodes[nodes.len() - 2])?
            };
            let args = convert_arguments(last)?;
            Ok(Expression::CallExpression(CallExpression {
                cv: None,
                callee: Box::new(callee),
                arguments: args,
            }))
        }
        _ if has_token(node, ".") => {
            // Member access: obj.name
            convert_member_expression(node)
        }
        _ if has_token(node, "[") => {
            // Computed member access: obj[key]
            convert_member_expression(node)
        }
        _ => {
            // Optional chain — Phase 2.
            Err(unsupported(node))
        }
    }
}

fn convert_arguments(node: &GrammarASTNode) -> Result<Vec<Expression>, BridgeError> {
    // arguments = LPAREN [ argument_list [ COMMA ] ] RPAREN
    // argument_list = argument { COMMA argument }
    // argument = [ ELLIPSIS ] assignment_expression
    let nodes = node_children(node);
    let mut args = Vec::new();
    for n in nodes {
        match n.rule_name.as_str() {
            "argument_list" => {
                for arg_n in node_children(n) {
                    args.push(convert_argument(arg_n)?);
                }
            }
            r if r.contains("expression") || r == "assignment_expression" => {
                args.push(convert_expression(n)?);
            }
            _ => {}
        }
    }
    Ok(args)
}

fn convert_argument(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // argument = [ ELLIPSIS ] assignment_expression
    if has_token(node, "...") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "SpreadElement".to_string(),
            location: loc(node),
        });
    }
    let n = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "argument: missing expression"))?;
    convert_expression(n)
}

// -------------------------------------------------------------------------
// member_expression
// -------------------------------------------------------------------------

fn convert_member_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // member_expression = primary_expression
    //                   | member_expression LBRACKET expression RBRACKET  (computed)
    //                   | member_expression DOT NAME                       (property)
    //                   | member_expression OPTIONAL_CHAINING ...          (unsupported)
    //                   | "new" member_expression arguments                (unsupported Phase 2)
    //                   | "super" ...                                      (unsupported)
    //
    // The grammar parser resolves left-recursion by returning the first match,
    // so for `a.b` children could be [Node(primary), Token("."), Token("b")]
    // and for `a[k]` they could be [Node(primary), Token("["), Node(expression), Token("]")].

    let nodes = node_children(node);

    if nodes.is_empty() {
        return Err(internal(node, "member_expression: no children"));
    }

    // Optional chain — Phase 2.
    if has_token(node, "?.") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "OptionalMemberExpression".to_string(),
            location: loc(node),
        });
    }

    // `new` expression — Phase 2.
    if node.children.iter().any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "new")) {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "NewExpression".to_string(),
            location: loc(node),
        });
    }

    // `super` — Phase 3.
    if node.children.iter().any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "super")) {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "Super".to_string(),
            location: loc(node),
        });
    }

    // A bare primary has a SINGLE child overall (just the
    // primary_expression Node, no suffix tokens). We check the full
    // children list, NOT just the Node children: `a.b` has one Node
    // child (`a`) but two suffix tokens (`.` and `b`), so counting
    // Nodes alone would wrongly treat `a.b` as a bare primary and
    // drop the `.b`. (That was the bug this guard previously had.)
    if node.children.len() == 1 {
        return convert_expression(nodes[0]); // primary_expression pass-through
    }

    // Suffix chain: `primary { DOT NAME | LBRACKET expr RBRACKET }`.
    //
    // The grammar's `member_expression = primary_expression { … }`
    // repetition produces a FLAT child list — the primary Node
    // followed by an arbitrary number of `.NAME` and `[expr]`
    // suffixes (e.g. `a.b.c`, `a[0].b`, `a.b[c].d`). We walk the raw
    // children left-to-right, folding each suffix onto the growing
    // `base`, exactly as `convert_optional_chain_expression` does for
    // its base member_expression. The previous single-suffix
    // implementation handled only one `.NAME` and silently dropped
    // the rest of the chain.
    let children = &node.children;

    // The base is the first child — always the primary_expression Node.
    let mut base = match children.first() {
        Some(ASTNodeOrToken::Node(n)) => convert_expression(n)?,
        _ => return Err(internal(node, "member_expression: expected primary base")),
    };

    let mut i = 1;
    while i < children.len() {
        match &children[i] {
            // `.NAME` — non-computed (dot) property access.
            ASTNodeOrToken::Token(t) if t.value == "." => {
                let prop_name = match children.get(i + 1) {
                    Some(ASTNodeOrToken::Token(name)) => name.value.clone(),
                    _ => {
                        return Err(internal(
                            node,
                            "member_expression.dot: missing property name",
                        ))
                    }
                };
                base = Expression::MemberExpression(MemberExpression {
                    cv: None,
                    object: Box::new(base),
                    property: Box::new(Expression::Identifier(Identifier {
                        cv: None,
                        name: prop_name,
                    })),
                    computed: false,
                });
                i += 2; // consume DOT + NAME
            }
            // `[expr]` — computed property access.
            ASTNodeOrToken::Token(t) if t.value == "[" => {
                // The key is the next Node child; skip to it.
                let key_node = children[i + 1..].iter().find_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                });
                let key = match key_node {
                    Some(n) => convert_expression(n)?,
                    None => {
                        return Err(internal(
                            node,
                            "member_expression.computed: missing key",
                        ))
                    }
                };
                base = Expression::MemberExpression(MemberExpression {
                    cv: None,
                    object: Box::new(base),
                    property: Box::new(key),
                    computed: true,
                });
                // Advance past `[ … ]`: find the matching RBRACKET.
                i += 1;
                while i < children.len() {
                    let is_rbracket = matches!(
                        &children[i],
                        ASTNodeOrToken::Token(t) if t.value == "]"
                    );
                    i += 1;
                    if is_rbracket {
                        break;
                    }
                }
            }
            // A tagged-template suffix on a member base is Phase 2.
            ASTNodeOrToken::Node(n) if n.rule_name == "template_literal" => {
                return Err(BridgeError::UnsupportedSyntax {
                    rule: "TaggedTemplateExpression".to_string(),
                    location: loc(n),
                });
            }
            // RBRACKET / NAME already consumed by their openers, and
            // any stray token is skipped defensively.
            _ => i += 1,
        }
    }

    Ok(base)
}

// -------------------------------------------------------------------------
// primary_expression
// -------------------------------------------------------------------------

fn convert_primary_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // primary_expression = "this" | NAME | NUMBER | BIGINT | STRING | REGEX
    //                    | "true" | "false" | "null" | array_literal
    //                    | object_literal | LPAREN expression RPAREN
    //                    | function_expression | generator_expression | ...
    //
    // Alternation: the parser's children are either a single Node (for rule
    // references like array_literal, object_literal, parenthesized expr) or
    // a single Token (for terminal alternatives like NAME, NUMBER, STRING,
    // keyword "true", keyword "false", keyword "null").
    //
    // Parenthesized `(expr)` produces [Token("("), Node("expression"), Token(")")]
    // — not a single child — so we handle it via the paren-detection path.

    // First: look for a child Node. Parenthesized expr has a Node but also
    // Tokens; non-paren rule refs have only a single Node.
    let nodes = node_children(node);
    if nodes.len() == 1 && !has_token(node, "(") {
        return convert_expression(nodes[0]);
    }
    // Parenthesized expression: (expr)
    if has_token(node, "(") {
        if let Some(inner_n) = nodes.first() {
            return convert_expression(inner_n);
        }
    }

    // Token leaf: a single token child.
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            return convert_primary_token(t, node);
        }
    }

    Err(internal(node, "primary_expression: unrecognised shape"))
}

/// Convert a single-token primary expression.
///
/// NUMBER/STRING/NAME are encoded in `t.type_` (not `t.type_name`).
/// BIGINT has `type_ = TokenType::Name` and `type_name = Some("BIGINT")`.
fn convert_primary_token(t: &Token, ctx: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // Value-based checks first (keywords: this, true, false, null, undefined).
    match t.value.as_str() {
        "this" => return Err(BridgeError::UnsupportedSyntax {
            rule: "ThisExpression".to_string(),
            location: loc(ctx),
        }),
        "null" => return Ok(Expression::NullLiteral(NullLiteral { cv: None })),
        "undefined" => return Ok(Expression::UndefinedLiteral(UndefinedLiteral { cv: None })),
        "true" => return Ok(Expression::BooleanLiteral(BooleanLiteral { cv: None, value: true })),
        "false" => return Ok(Expression::BooleanLiteral(BooleanLiteral { cv: None, value: false })),
        _ => {}
    }

    // BIGINT: type_ == TokenType::Name but type_name == Some("BIGINT").
    if t.type_name.as_deref() == Some("BIGINT") {
        let raw = t.value.clone();
        let value = raw.trim_end_matches('n').to_string();
        return Ok(Expression::BigIntLiteral(BigIntLiteral { cv: None, value, raw }));
    }

    // Standard terminal types via the type_ discriminant.
    match t.type_ {
        TokenType::Number => {
            let val: f64 = parse_js_number(&t.value).map_err(|_| {
                BridgeError::InternalError {
                    msg: format!("failed to parse numeric literal '{}'", t.value),
                    rule: ctx.rule_name.clone(),
                }
            })?;
            return Ok(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: val,
                raw: t.value.clone(),
            }));
        }
        TokenType::String => {
            let raw = t.value.clone();
            let value = unquote_string(&raw);
            return Ok(Expression::StringLiteral(StringLiteral { cv: None, value, raw }));
        }
        TokenType::Name => {
            // Plain identifier (variable name or non-keyword reference).
            return Ok(Expression::Identifier(Identifier { cv: None, name: t.value.clone() }));
        }
        TokenType::Keyword => {
            // Context keyword used as an expression (e.g. `undefined` is not
            // reserved in ES5 and is tokenised as Name, but other keywords
            // like `super`, `new.target` land here; those are Phase 2/3).
            return Err(BridgeError::UnsupportedSyntax {
                rule: format!("keyword-expression:{}", t.value),
                location: loc(ctx),
            });
        }
        _ => {}
    }

    // Fallback: treat as identifier (catches edge cases like regex literals
    // that look like punctuation in some grammars).
    Ok(Expression::Identifier(Identifier { cv: None, name: t.value.clone() }))
}

// =========================================================================
// Literals
// =========================================================================

fn convert_array_literal(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // array_literal = LBRACKET [ element_list ] RBRACKET ;
    // element_list = [ ELLIPSIS ] assignment_expression { COMMA [ ELLIPSIS ] assignment_expression }
    let nodes = node_children(node);
    let mut elements = Vec::new();
    for n in nodes {
        match n.rule_name.as_str() {
            "element_list" => {
                for elem_n in node_children(n) {
                    if has_token(elem_n, "...") {
                        return Err(BridgeError::UnsupportedSyntax {
                            rule: "SpreadElement".to_string(),
                            location: loc(elem_n),
                        });
                    }
                    let child = node_children(elem_n).into_iter().next()
                        .unwrap_or(elem_n);
                    elements.push(Some(convert_expression(child)?));
                }
            }
            r if r.contains("expression") => {
                elements.push(Some(convert_expression(n)?));
            }
            _ => {}
        }
    }
    Ok(Expression::ArrayExpression(ArrayExpression { cv: None, elements }))
}

fn convert_object_literal(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // object_literal = LBRACE [ property_definition { COMMA property_definition } ] RBRACE
    let nodes = node_children(node);
    let mut properties = Vec::new();
    for n in nodes {
        match n.rule_name.as_str() {
            "property_definition" => {
                properties.push(convert_property_definition(n)?);
            }
            _ => {}
        }
    }
    Ok(Expression::ObjectExpression(ObjectExpression { cv: None, properties }))
}

fn convert_property_definition(node: &GrammarASTNode) -> Result<Property, BridgeError> {
    // property_definition = property_name COLON assignment_expression
    //                     | NAME  (shorthand)
    //                     | ELLIPSIS assignment_expression  (spread — unsupported)
    //                     | method_definition  (unsupported Phase 2)
    if has_token(node, "...") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "SpreadProperty".to_string(),
            location: loc(node),
        });
    }
    let nodes = node_children(node);
    if nodes.len() == 2 {
        // property_name : value
        let key_n = nodes[0];
        let val_n = nodes[1];
        let key = convert_property_key(key_n)?;
        let value = convert_expression(val_n)?;
        return Ok(Property {
            cv: None,
            key,
            value: Box::new(value),
            kind: PropertyKind::Init,
            shorthand: false,
            computed: false,
            method: false,
        });
    }
    if nodes.is_empty() {
        // Shorthand NAME property: { x } == { x: x }
        let name = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t.value.clone()),
            _ => None,
        });
        let name = name.ok_or_else(|| internal(node, "property_definition: missing name"))?;
        let id = Expression::Identifier(Identifier { cv: None, name: name.clone() });
        return Ok(Property {
            cv: None,
            key: PropertyKey::Identifier(Identifier { cv: None, name }),
            value: Box::new(id),
            kind: PropertyKind::Init,
            shorthand: true,
            computed: false,
            method: false,
        });
    }
    Err(unsupported(node))
}

fn convert_property_key(node: &GrammarASTNode) -> Result<PropertyKey, BridgeError> {
    // property_name = NAME | STRING | NUMBER | LBRACKET assignment_expression RBRACKET
    if has_token(node, "[") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "ComputedPropertyKey".to_string(),
            location: loc(node),
        });
    }
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if let Some(ref tn) = t.type_name {
                match tn.as_str() {
                    "STRING" => {
                        let raw = t.value.clone();
                        let value = unquote_string(&raw);
                        return Ok(PropertyKey::StringLiteral(
                            coding_adventures_javascript_ast::expression::StringLiteral { cv: None, value, raw }
                        ));
                    }
                    "NUMBER" => {
                        let val: f64 = parse_js_number(&t.value).unwrap_or(0.0);
                        return Ok(PropertyKey::NumericLiteral(
                            coding_adventures_javascript_ast::expression::NumericLiteral { cv: None, value: val, raw: t.value.clone() }
                        ));
                    }
                    _ => {}
                }
            }
            return Ok(PropertyKey::Identifier(Identifier { cv: None, name: t.value.clone() }));
        }
    }
    Err(internal(node, "property_name: no key token"))
}

// =========================================================================
// String / number utilities
// =========================================================================

/// Parse a JavaScript numeric literal string to `f64`, handling all JS
/// radix forms so no literal produces a silently-wrong value.
fn parse_js_number(s: &str) -> Result<f64, ()> {
    // Strip numeric separators (ES2021): `1_000` → `1000`
    let cleaned: String = s.chars().filter(|&c| c != '_').collect();
    // Hex: 0x / 0X
    if let Some(hex) = cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).map(|n| n as f64).map_err(|_| ());
    }
    // Binary: 0b / 0B
    if let Some(bin) = cleaned.strip_prefix("0b").or_else(|| cleaned.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2).map(|n| n as f64).map_err(|_| ());
    }
    // Octal: 0o / 0O (modern) or legacy 0NNN (digits only, first digit is 0)
    if let Some(oct) = cleaned.strip_prefix("0o").or_else(|| cleaned.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8).map(|n| n as f64).map_err(|_| ());
    }
    // Legacy octal: 0NNN (all octal digits, no decimal point)
    if cleaned.starts_with('0') && cleaned.len() > 1
        && !cleaned.contains('.')
        && !cleaned.to_ascii_lowercase().contains('e')
        && cleaned[1..].chars().all(|c| c.is_ascii_digit() && c < '8')
    {
        return u64::from_str_radix(&cleaned[1..], 8).map(|n| n as f64).map_err(|_| ());
    }
    // Decimal / float / scientific.
    cleaned.parse::<f64>().map_err(|_| ())
}

/// Strip the surrounding quotes and unescape the content of a JS string token.
/// Returns the unescaped content (e.g. `"hello\nworld"` → `hello` + newline + `world`).
fn unquote_string(raw: &str) -> String {
    let s = if (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''))
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some('\'') => result.push('\''),
            Some('"') => result.push('"'),
            Some('0') => result.push('\0'),
            Some('b') => result.push('\x08'),
            Some('f') => result.push('\x0C'),
            Some('v') => result.push('\x0B'),
            // \xHH — two hex digits.
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(Some(ch)) = u32::from_str_radix(&hex, 16).map(char::from_u32) {
                    result.push(ch);
                } else {
                    result.push('\\');
                    result.push('x');
                    result.push_str(&hex);
                }
            }
            // \uXXXX or \u{XXXXX} — Unicode escapes.
            Some('u') => {
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let hex: String = chars.by_ref().take_while(|&c| c != '}').collect();
                    if let Ok(Some(ch)) = u32::from_str_radix(&hex, 16).map(char::from_u32) {
                        result.push(ch);
                    } else {
                        result.push_str("\\u{");
                        result.push_str(&hex);
                        result.push('}');
                    }
                } else {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(Some(ch)) = u32::from_str_radix(&hex, 16).map(char::from_u32) {
                        result.push(ch);
                    } else {
                        result.push('\\');
                        result.push('u');
                        result.push_str(&hex);
                    }
                }
            }
            // Line continuation: backslash followed by newline.
            Some('\n') | Some('\r') => {}
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_javascript_typed, DEFAULT_ES_VERSION};
    use coding_adventures_javascript_tokens::EsVersion;

    fn bridge(src: &str) -> Result<Program, BridgeError> {
        let node = parse_javascript_typed(src, DEFAULT_ES_VERSION).expect("parse failed");
        grammar_to_program(&node, DEFAULT_ES_VERSION)
    }

    fn bridge_ok(src: &str) -> Program {
        bridge(src).unwrap_or_else(|e| panic!("bridge failed for {:?}: {e}", src))
    }

    // -----------------------------------------------------------------------
    // Empty program
    // -----------------------------------------------------------------------

    #[test]
    fn empty_program() {
        let p = bridge_ok("");
        assert!(p.body.is_empty());
    }

    // -----------------------------------------------------------------------
    // Literals
    // -----------------------------------------------------------------------

    #[test]
    fn numeric_literal() {
        let p = bridge_ok("42;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(s)) => {
                if let coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es) = s {
                    assert!(matches!(&es.expression, Expression::NumericLiteral(n) if n.value == 42.0));
                } else { panic!("expected ExpressionStatement") }
            }
            _ => panic!("expected Statement"),
        }
    }

    #[test]
    fn string_literal() {
        let p = bridge_ok("\"hello\";");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::StringLiteral(s) if s.value == "hello"));
            }
            _ => panic!("expected ExpressionStatement"),
        }
    }

    #[test]
    fn boolean_literal_true() {
        let p = bridge_ok("true;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::BooleanLiteral(b) if b.value));
            }
            _ => panic!("expected ExpressionStatement"),
        }
    }

    #[test]
    fn null_literal() {
        let p = bridge_ok("null;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::NullLiteral(_)));
            }
            _ => panic!("expected ExpressionStatement"),
        }
    }

    // -----------------------------------------------------------------------
    // Variable declarations
    // -----------------------------------------------------------------------

    #[test]
    fn var_declaration() {
        let p = bridge_ok("var x = 1;");
        assert_eq!(p.body.len(), 1);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Declaration(
                Declaration::VariableDeclaration(v)
            )) => {
                assert_eq!(v.kind, VarKind::Var);
                assert_eq!(v.declarations.len(), 1);
                assert!(matches!(&v.declarations[0].id, BindingTarget::Identifier(id) if id.name == "x"));
            }
            _ => panic!("expected VariableDeclaration"),
        }
    }

    #[test]
    fn let_declaration() {
        let p = bridge_ok("let y = 2;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Declaration(
                Declaration::VariableDeclaration(v)
            )) => {
                assert_eq!(v.kind, VarKind::Let);
            }
            _ => panic!("expected VariableDeclaration"),
        }
    }

    #[test]
    fn const_declaration() {
        let p = bridge_ok("const z = 3;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Declaration(
                Declaration::VariableDeclaration(v)
            )) => {
                assert_eq!(v.kind, VarKind::Const);
            }
            _ => panic!("expected VariableDeclaration"),
        }
    }

    // -----------------------------------------------------------------------
    // Binary expressions
    // -----------------------------------------------------------------------

    #[test]
    fn binary_add() {
        let p = bridge_ok("1 + 2;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::BinaryExpression(b) if b.operator == BinaryOperator::Add));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn logical_and() {
        let p = bridge_ok("a && b;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::LogicalExpression(l) if l.operator == LogicalOperator::And));
            }
            _ => panic!(),
        }
    }

    // -----------------------------------------------------------------------
    // If statement
    // -----------------------------------------------------------------------

    #[test]
    fn if_statement_no_else() {
        let p = bridge_ok("if (x) { y; }");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::IfStatement(s)
            )) => {
                assert!(s.alternate.is_none());
            }
            _ => panic!("expected IfStatement"),
        }
    }

    #[test]
    fn if_statement_with_else() {
        let p = bridge_ok("if (x) { y; } else { z; }");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::IfStatement(s)
            )) => {
                assert!(s.alternate.is_some());
            }
            _ => panic!("expected IfStatement"),
        }
    }

    // -----------------------------------------------------------------------
    // Function declaration
    // -----------------------------------------------------------------------

    #[test]
    fn function_declaration() {
        let p = bridge_ok("function foo(x) { return x; }");
        match &p.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) => {
                assert_eq!(f.id.name, "foo");
                assert_eq!(f.params.len(), 1);
                assert!(!f.generator);
                assert!(!f.is_async);
            }
            _ => panic!("expected FunctionDeclaration"),
        }
    }

    // -----------------------------------------------------------------------
    // Return statement
    // -----------------------------------------------------------------------

    #[test]
    fn return_with_value() {
        let p = bridge_ok("function f() { return 42; }");
        match &p.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) => {
                match &f.body.body[0] {
                    Statement::Tagged(
                        coding_adventures_javascript_ast::statement::TaggedStatement::ReturnStatement(r)
                    ) => {
                        assert!(r.argument.is_some());
                    }
                    _ => panic!("expected ReturnStatement"),
                }
            }
            _ => panic!(),
        }
    }

    // -----------------------------------------------------------------------
    // Unsupported syntax gracefully errors
    // -----------------------------------------------------------------------

    #[test]
    fn do_while_is_unsupported() {
        let result = bridge("do { } while (true);");
        assert!(matches!(result, Err(BridgeError::UnsupportedSyntax { .. })));
    }

    #[test]
    fn call_expression_roundtrip() {
        let p = bridge_ok("foo(1, 2);");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::CallExpression(c) if c.arguments.len() == 2));
            }
            _ => panic!("expected call expression"),
        }
    }

    #[test]
    fn while_statement_bridge() {
        // Note: standalone assignment expressions (`x = expr;`) are not yet
        // parseable by the grammar parser (ordered alternation matches the
        // `conditional_expression` alternative first). Use a call expression
        // body instead to exercise the while loop structure.
        let p = bridge_ok("while (x > 0) { foo(); }");
        assert!(matches!(
            &p.body[0],
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::WhileStatement(_)
            ))
        ));
    }

    #[test]
    fn switch_statement_bridge() {
        let p = bridge_ok("switch (x) { case 1: break; default: break; }");
        assert!(matches!(
            &p.body[0],
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::SwitchStatement(_)
            ))
        ));
    }

    // -----------------------------------------------------------------------
    // member_expression — dot and computed property chains
    //
    // Regression coverage for the bug where `convert_member_expression`
    // early-returned on `nodes.len() == 1` (Node children only), counting
    // the single primary Node and ignoring the `.NAME` suffix tokens — so
    // `a.b` collapsed to `a` and `a.b.c` collapsed to `a.c`. The grammar's
    // `member_expression = primary_expression { DOT NAME | LBRACKET … }`
    // repetition is a flat suffix list that must be walked in full.
    // -----------------------------------------------------------------------

    /// Pull the expression out of a single-statement program whose body is
    /// `<expr>;`.
    fn first_expr(p: &Program) -> &Expression {
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es),
            )) => &es.expression,
            _ => panic!("expected an expression statement"),
        }
    }

    #[test]
    fn member_dot_single() {
        // `a.b` — the property `b` must survive (the original bug dropped it).
        let p = bridge_ok("a.b;");
        match first_expr(&p) {
            Expression::MemberExpression(m) => {
                assert!(!m.computed, "dot access is not computed");
                assert!(matches!(&*m.object, Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(&*m.property, Expression::Identifier(i) if i.name == "b"));
            }
            other => panic!("expected MemberExpression, got {other:?}"),
        }
    }

    #[test]
    fn member_dot_chain() {
        // `a.b.c` — left-associative: ((a.b).c). Both suffixes must survive.
        let p = bridge_ok("a.b.c;");
        match first_expr(&p) {
            Expression::MemberExpression(outer) => {
                assert!(matches!(&*outer.property, Expression::Identifier(i) if i.name == "c"));
                match &*outer.object {
                    Expression::MemberExpression(inner) => {
                        assert!(matches!(&*inner.object, Expression::Identifier(i) if i.name == "a"));
                        assert!(matches!(&*inner.property, Expression::Identifier(i) if i.name == "b"));
                    }
                    other => panic!("expected inner MemberExpression a.b, got {other:?}"),
                }
            }
            other => panic!("expected MemberExpression, got {other:?}"),
        }
    }

    #[test]
    fn member_computed_then_dot() {
        // `a[0].b` — a computed access followed by a dot access.
        let p = bridge_ok("a[0].b;");
        match first_expr(&p) {
            Expression::MemberExpression(outer) => {
                assert!(!outer.computed);
                assert!(matches!(&*outer.property, Expression::Identifier(i) if i.name == "b"));
                match &*outer.object {
                    Expression::MemberExpression(inner) => {
                        assert!(inner.computed, "[0] is computed");
                        assert!(matches!(&*inner.object, Expression::Identifier(i) if i.name == "a"));
                        assert!(matches!(&*inner.property, Expression::NumericLiteral(n) if n.value == 0.0));
                    }
                    other => panic!("expected inner computed member a[0], got {other:?}"),
                }
            }
            other => panic!("expected MemberExpression, got {other:?}"),
        }
    }

    #[test]
    fn member_dot_then_computed() {
        // `a.b[c]` — a dot access followed by a computed access.
        let p = bridge_ok("a.b[c];");
        match first_expr(&p) {
            Expression::MemberExpression(outer) => {
                assert!(outer.computed, "[c] is computed");
                assert!(matches!(&*outer.property, Expression::Identifier(i) if i.name == "c"));
                match &*outer.object {
                    Expression::MemberExpression(inner) => {
                        assert!(!inner.computed);
                        assert!(matches!(&*inner.object, Expression::Identifier(i) if i.name == "a"));
                        assert!(matches!(&*inner.property, Expression::Identifier(i) if i.name == "b"));
                    }
                    other => panic!("expected inner dot member a.b, got {other:?}"),
                }
            }
            other => panic!("expected MemberExpression, got {other:?}"),
        }
    }

    #[test]
    fn member_method_call_keeps_property() {
        // `a.b(c)` — the callee must be the member `a.b`, not bare `a`
        // (the bug surfaced as `console.log(x)` emitting `console(x)`).
        let p = bridge_ok("a.b(c);");
        match first_expr(&p) {
            Expression::CallExpression(call) => {
                assert_eq!(call.arguments.len(), 1);
                match &*call.callee {
                    Expression::MemberExpression(m) => {
                        assert!(matches!(&*m.object, Expression::Identifier(i) if i.name == "a"));
                        assert!(matches!(&*m.property, Expression::Identifier(i) if i.name == "b"));
                    }
                    other => panic!("expected member callee a.b, got {other:?}"),
                }
            }
            other => panic!("expected CallExpression, got {other:?}"),
        }
    }
}
