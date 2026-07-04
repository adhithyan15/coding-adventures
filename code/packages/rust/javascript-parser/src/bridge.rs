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
        ArrayExpression, ArrowBody, ArrowFunctionExpression, AssignmentExpression,
        AssignmentOperator, AssignmentTarget,
        BigIntLiteral, BinaryExpression, BinaryOperator, BooleanLiteral, CallExpression,
        ConditionalExpression, Expression, FunctionExpression, Identifier, LogicalExpression,
        LogicalOperator,
        MemberExpression, NewExpression, NullLiteral, NumericLiteral, ObjectExpression, Property,
        PropertyKey, PropertyKind, SequenceExpression, SpreadElement, StringLiteral, TaggedTemplateExpression,
        TemplateElement, TemplateLiteral,
        UnaryExpression, UnaryOperator,
        ThisExpression, UndefinedLiteral, UpdateExpression, UpdateOperator, YieldExpression,
    },
    statement::{
        BlockStatement, BreakStatement, CatchClause, ContinueStatement, DebuggerStatement,
        DoWhileStatement, EmptyStatement, ExpressionStatement, ForInStatement, ForInit,
        ForOfStatement, ForStatement, IfStatement, LabeledStatement, ReturnStatement, Statement,
        SwitchCase, SwitchStatement, ThrowStatement, TryStatement, WhileStatement,
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
        // `function_declaration` and `generator_declaration` share the same
        // converter — the `*` distinguishes them and sets the `generator` flag
        // (CLOC12.163 PR2).
        "function_declaration" | "generator_declaration" => {
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
        "do_while_statement" => {
            convert_do_while_statement(child).map(Statement::do_while_statement)
        }
        "for_statement" => convert_for_statement(child).map(Statement::for_statement),
        "for_in_statement" => convert_for_in_statement(child).map(Statement::for_in_statement),
        "for_of_statement" => convert_for_of_statement(child).map(Statement::for_of_statement),
        "continue_statement" => convert_continue_statement(child).map(Statement::continue_statement),
        "break_statement" => convert_break_statement(child).map(Statement::break_statement),
        "return_statement" => convert_return_statement(child).map(Statement::return_statement),
        "switch_statement" => convert_switch_statement(child).map(Statement::switch_statement),
        "labelled_statement" => convert_labeled_statement(child).map(Statement::labeled_statement),
        "throw_statement" => convert_throw_statement(child).map(Statement::throw_statement),
        "try_statement" => convert_try_statement(child).map(Statement::try_statement),
        // debugger_statement = "debugger" SEMICOLON — no node children, so the
        // typed node is a bare marker. (CLOC21.)
        "debugger_statement" => Ok(Statement::debugger_statement(DebuggerStatement { cv: None })),
        // Phase 2+ — not yet in the typed AST
        "for_await_of_statement" | "with_statement" | "using_declaration"
        | "await_using_declaration" => Err(unsupported(child)),
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

fn convert_do_while_statement(node: &GrammarASTNode) -> Result<DoWhileStatement, BridgeError> {
    // do_while_statement = "do" statement "while" LPAREN expression RPAREN [";"]
    // Node children (tokens filtered out): [statement, expression] — the body
    // comes first in source order, the test second. This is the mirror of
    // while_statement, which is [expression, statement].
    let nodes = node_children(node);
    if nodes.len() < 2 {
        return Err(internal(node, "do_while_statement needs 2 node children"));
    }
    Ok(DoWhileStatement {
        cv: None,
        body: Box::new(convert_statement(nodes[0])?),
        test: convert_expression(nodes[1])?,
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

fn convert_for_in_statement(node: &GrammarASTNode) -> Result<ForInStatement, BridgeError> {
    // for_in_statement = "for" LPAREN
    //   ( "var" variable_declaration | "let" binding_element
    //   | "const" binding_element | left_hand_side_expression )
    //   "in" expression RPAREN statement ;
    //
    // Walk children using the `in` and `)` tokens as phase delimiters:
    //   phase 0 = the left (before `in`); phase 1 = the right expression
    //   (between `in` and `)`); phase 2 = the body (after `)`). The
    //   `var`/`let`/`const` keyword (if present) appears as a phase-0 token.
    let mut phase = 0usize;
    let mut left_kind: Option<VarKind> = None;
    let mut phase_nodes: [Vec<&GrammarASTNode>; 3] = [vec![], vec![], vec![]];

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if phase == 0 && t.value == "in" => phase = 1,
            ASTNodeOrToken::Token(t) if phase == 1 && t.value == ")" => phase = 2,
            ASTNodeOrToken::Token(t) if phase == 0 => match t.value.as_str() {
                "var" => left_kind = Some(VarKind::Var),
                "let" => left_kind = Some(VarKind::Let),
                "const" => left_kind = Some(VarKind::Const),
                _ => {}
            },
            ASTNodeOrToken::Node(n) => phase_nodes[phase.min(2)].push(n),
            ASTNodeOrToken::Token(_) => {}
        }
    }

    // Left: a binding (var/let/const) or an existing assignment target.
    let left_node = *phase_nodes[0]
        .first()
        .ok_or_else(|| internal(node, "for_in_statement: missing left"))?;
    let left = match left_kind {
        Some(kind) => {
            // A single declarator with no initializer. `convert_variable_declarator`
            // declines destructuring (`binding_pattern`) by returning
            // `UnsupportedSyntax`. We additionally map any other conversion
            // failure to a graceful decline (whitespace-only fallback) rather
            // than a hard error, so an unrepresentable binding shape never
            // aborts compilation.
            let declarator = convert_variable_declarator(left_node)
                .map_err(|_| unsupported(left_node))?;
            ForInit::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind,
                declarations: vec![declarator],
            })
        }
        None => ForInit::Expression(convert_expression(left_node)?),
    };

    // Right: the enumerated expression.
    let right_node = *phase_nodes[1]
        .first()
        .ok_or_else(|| internal(node, "for_in_statement: missing right expression"))?;
    let right = convert_expression(right_node)?;

    // Body.
    let body_node = *phase_nodes[2]
        .first()
        .ok_or_else(|| internal(node, "for_in_statement: missing body"))?;
    let body = Box::new(convert_statement(body_node)?);

    Ok(ForInStatement {
        cv: None,
        left,
        right,
        body,
    })
}

fn convert_for_of_statement(node: &GrammarASTNode) -> Result<ForOfStatement, BridgeError> {
    // for_of_statement = "for" LPAREN
    //   ( "var" variable_declaration | "let" binding_element
    //   | "const" binding_element | "using" binding_element
    //   | left_hand_side_expression )
    //   "of" assignment_expression RPAREN statement ;
    //
    // Structurally identical to for_in_statement, but the phase delimiter is
    // the `of` token (not `in`). A `using` binding declaration is not
    // represented — we decline it (graceful WHITESPACE_ONLY fallback).
    let mut phase = 0usize;
    let mut left_kind: Option<VarKind> = None;
    let mut saw_using = false;
    let mut phase_nodes: [Vec<&GrammarASTNode>; 3] = [vec![], vec![], vec![]];

    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(t) if phase == 0 && t.value == "of" => phase = 1,
            ASTNodeOrToken::Token(t) if phase == 1 && t.value == ")" => phase = 2,
            ASTNodeOrToken::Token(t) if phase == 0 => match t.value.as_str() {
                "var" => left_kind = Some(VarKind::Var),
                "let" => left_kind = Some(VarKind::Let),
                "const" => left_kind = Some(VarKind::Const),
                "using" => saw_using = true,
                _ => {}
            },
            ASTNodeOrToken::Node(n) => phase_nodes[phase.min(2)].push(n),
            ASTNodeOrToken::Token(_) => {}
        }
    }

    // `using` declarations are not modelled — decline gracefully.
    if saw_using {
        return Err(unsupported(node));
    }

    // Left: a binding (var/let/const) or an existing assignment target.
    let left_node = *phase_nodes[0]
        .first()
        .ok_or_else(|| internal(node, "for_of_statement: missing left"))?;
    let left = match left_kind {
        Some(kind) => {
            // A single declarator with no initializer. `convert_variable_declarator`
            // declines destructuring; any other unrepresentable shape is mapped
            // to a graceful decline rather than a hard error.
            let declarator =
                convert_variable_declarator(left_node).map_err(|_| unsupported(left_node))?;
            ForInit::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind,
                declarations: vec![declarator],
            })
        }
        None => ForInit::Expression(convert_expression(left_node)?),
    };

    // Right: the iterable expression (an assignment_expression).
    let right_node = *phase_nodes[1]
        .first()
        .ok_or_else(|| internal(node, "for_of_statement: missing right expression"))?;
    let right = convert_expression(right_node)?;

    // Body.
    let body_node = *phase_nodes[2]
        .first()
        .ok_or_else(|| internal(node, "for_of_statement: missing body"))?;
    let body = Box::new(convert_statement(body_node)?);

    Ok(ForOfStatement {
        cv: None,
        left,
        right,
        body,
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
// try_statement / catch_clause (CLOC19)
// -------------------------------------------------------------------------

fn convert_try_statement(node: &GrammarASTNode) -> Result<TryStatement, BridgeError> {
    // try_statement = "try" block ( catch_clause [ finally_clause ]
    //                             | finally_clause ) ;
    // Groups flatten, so the Node children are, in order: the try `block`,
    // then an optional `catch_clause` and/or `finally_clause`.
    let children = node_children(node);
    let block_n = children
        .first()
        .filter(|n| n.rule_name == "block")
        .ok_or_else(|| internal(node, "try_statement: missing try block"))?;
    let block = convert_block_statement(block_n)?;

    let mut handler = None;
    let mut finalizer = None;
    for n in children.iter().skip(1) {
        match n.rule_name.as_str() {
            "catch_clause" => handler = Some(convert_catch_clause(n)?),
            "finally_clause" => {
                // finally_clause = "finally" block
                let fb = node_children(n)
                    .into_iter()
                    .find(|c| c.rule_name == "block")
                    .ok_or_else(|| internal(n, "finally_clause: missing block"))?;
                finalizer = Some(convert_block_statement(fb)?);
            }
            _ => {}
        }
    }

    Ok(TryStatement {
        cv: None,
        block,
        handler,
        finalizer,
    })
}

fn convert_catch_clause(node: &GrammarASTNode) -> Result<CatchClause, BridgeError> {
    // catch_clause = "catch" [ LPAREN NAME RPAREN ] block ;
    // The grammar restricts the binding to a simple NAME (no destructuring),
    // so the optional param is the single token that is neither the `catch`
    // keyword nor a paren. Missing ⇒ the ES2019 optional-catch-binding form
    // `catch { … }`.
    let param = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t)
            if t.value != "catch" && t.value != "(" && t.value != ")" =>
        {
            Some(Identifier {
                cv: None,
                name: t.value.clone(),
            })
        }
        _ => None,
    });
    let body_n = node_children(node)
        .into_iter()
        .find(|c| c.rule_name == "block")
        .ok_or_else(|| internal(node, "catch_clause: missing body block"))?;
    let body = convert_block_statement(body_n)?;
    Ok(CatchClause {
        cv: None,
        param,
        body,
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

    // Destructuring binding patterns (`var [a, b] = c`, `let {p, q} = o`,
    // `const [x] = y`) are not yet supported by the typed bridge (Phase 2).
    // DECLINE GRACEFULLY here — return `unsupported` so the CLI falls back to
    // WHITESPACE_ONLY and still emits valid output (the same way spread,
    // optional chaining, `new`, etc. are declined).
    //
    // This check MUST come before the NAME lookup below. A binding pattern is
    // a `binding_pattern` *node* with no NAME *token* among the declarator's
    // direct children, so the `find_map` yields `None` and the
    // `ok_or_else(internal(...))` would abort the WHOLE compile with a hard
    // error (`exit 2`) instead of declining. Previously this check sat AFTER
    // that unwrap and was dead code for the destructuring case, so
    // `var [a,b]=c;` / `let {p,q}=o;` failed to compile at SIMPLE/ADVANCED.
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "binding_pattern" {
                return Err(unsupported(n));
            }
        }
    }

    // Find the NAME token (the binding identifier).
    let id_name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.value != "=" => Some(t.value.clone()),
        _ => None,
    });
    let id_name = id_name.ok_or_else(|| internal(node, "variable declarator: missing name"))?;

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
    // function_declaration  = "function"     NAME LPAREN [ formal_parameters ] RPAREN
    //                         LBRACE function_body RBRACE ;
    // generator_declaration = "function" "*" NAME LPAREN [ formal_parameters ] RPAREN
    //                         LBRACE function_body RBRACE ;
    // Token children include "function", (optional "*"), NAME, "(", ")", "{", "}"
    // Node children: optional formal_parameters, then function_body
    //
    // The two rules share this converter: a `*` token marks a generator
    // (CLOC12.163 PR2). We skip it during name extraction and record it in the
    // `generator` flag so the emitter re-prints `function*`.

    // Extract function name from token children (skip "function" and the
    // generator "*").
    let name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t)
            if t.value != "function"
                && t.value != "*"
                && t.value != "("
                && t.value != ")"
                && t.value != "{"
                && t.value != "}" =>
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
        generator: has_token(node, "*"),
        is_async: false,
    })
}

fn convert_function_expression(node: &GrammarASTNode) -> Result<FunctionExpression, BridgeError> {
    // function_expression  = "function"     [ NAME ] LPAREN [ formal_parameters ] RPAREN
    //                        LBRACE function_body RBRACE ;
    // generator_expression = "function" "*" [ NAME ] LPAREN [ formal_parameters ] RPAREN
    //                        LBRACE function_body RBRACE ;
    // Structurally identical to `function_declaration` (see above) with ONE
    // difference: the NAME is OPTIONAL. `function () {}` is anonymous;
    // `function f () {}` in value position binds `f` only inside its own body
    // (a body-local self-reference for recursion), never in the enclosing
    // scope. So a missing name is NOT an error here — it's the common case.
    // As with declarations, a `*` marks a generator expression (CLOC12.163 PR2):
    // skip it during name extraction and record it in `generator`.
    let name = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t)
            if t.value != "function"
                && t.value != "*"
                && t.value != "("
                && t.value != ")"
                && t.value != "{"
                && t.value != "}" =>
        {
            Some(t.value.clone())
        }
        _ => None,
    });

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

    Ok(FunctionExpression {
        cv: None,
        id: name.map(|name| Identifier { cv: None, name }),
        params,
        body,
        generator: has_token(node, "*"),
        is_async: false,
    })
}

// ---------------------------------------------------------------------
// YieldExpression (CLOC12.163 PR2 — bridge enable, gap-164)
// ---------------------------------------------------------------------

/// Convert a `yield_expression` grammar node into a [`YieldExpression`].
///
/// The parse tree has one of two shapes (confirmed by dumping the grammar
/// parser's output for `function*g(){yield x}` / `function*g(){yield* xs}`):
///
/// ```text
///   yield x     yield_expression = [ Token("yield"),            Node(assignment_expression) ]
///   yield* xs   yield_expression = [ Token("yield"), Token("*"), Node(assignment_expression) ]
/// ```
///
/// So `delegate` is simply "does the node carry a `*` token", and the operand
/// is the sole child *node* (`node_children` skips the `yield` / `*` tokens).
///
/// **Operand is mandatory in this grammar.** A bare operand-less `yield` does
/// *not* parse today — the grammar's `yield_expression` production requires an
/// `assignment_expression` operand (`function*g(){yield;}` is a parse error).
/// The typed AST models `argument` as `Option` (a bare `yield` is legal ES),
/// but the bridge only ever produces `Some(_)` until the grammar admits the
/// operand-less form. If a future grammar change yields an operand-less node,
/// the `node_children().next()` below returns `None` and we surface an internal
/// error rather than silently mis-converting.
fn convert_yield_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    let delegate = has_token(node, "*");
    let operand = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "yield_expression: missing operand"))?;
    Ok(Expression::YieldExpression(YieldExpression {
        cv: None,
        delegate,
        argument: Some(Box::new(convert_expression(operand)?)),
    }))
}

// ---------------------------------------------------------------------
// ArrowFunctionExpression (CLOC12.152 — bridge enable)
// ---------------------------------------------------------------------

/// Convert an `arrow_function` grammar node into an
/// [`ArrowFunctionExpression`].
///
/// ```text
///   arrow_function   = arrow_parameters ARROW concise_body ;
///   arrow_parameters = NAME | LPAREN [ formal_parameters ] RPAREN ;
///   concise_body     = assignment_expression | LBRACE function_body RBRACE ;
/// ```
///
/// # Two grammar limitations this bridge works around
///
/// 1. **Block bodies don't parse (gap-156).** The current ECMAScript
///    grammar rejects a *statement* block body — `x => { return x; }`
///    fails to parse outright — so the parser only ever produces a
///    **concise** (expression) `concise_body`. This converter therefore
///    always yields [`ArrowBody::Expression`]; the emitter and passes
///    already model [`ArrowBody::Block`] for when the grammar is fixed.
/// 2. **`() => {}` mis-parses as an empty-*object* concise body.** Because
///    the block alternative isn't taken, the grammar reads the braces of
///    `() => {}` as an empty **object literal** returned by a concise body
///    (`() => ({})`), which is a *different value* than the real
///    empty-block arrow (which returns `undefined`). We cannot tell the
///    two apart from the parse tree — the distinguishing information was
///    already lost — so to avoid a **miscompile** we DECLINE any arrow
///    whose concise body is an object literal (falling back to
///    whitespace-only, which re-emits the source unchanged and is always
///    correct). Genuine object-returning arrows (`x => ({a:1})`) decline
///    too; that only forgoes an optimisation, never correctness.
///
/// Async arrows (`async x => x`) parse under the separate
/// `async_arrow_function` rule and remain declined for now — a follow-up
/// once the async evaluation model lands.
fn convert_arrow_function(node: &GrammarASTNode) -> Result<ArrowFunctionExpression, BridgeError> {
    let children = node_children(node);

    let mut params = Vec::new();
    let mut body: Option<ArrowBody> = None;

    for n in &children {
        match n.rule_name.as_str() {
            "arrow_parameters" => params = convert_arrow_parameters(n)?,
            "concise_body" => body = Some(convert_concise_body(n)?),
            _ => {}
        }
    }

    let body = body.ok_or_else(|| internal(node, "arrow_function: missing concise_body"))?;

    // Guard against the `() => {}` ambiguity described above: an
    // object-literal concise body cannot be distinguished from an empty
    // block body, so decline rather than risk a miscompile.
    if let ArrowBody::Expression(e) = &body {
        if matches!(**e, Expression::ObjectExpression(_)) {
            return Err(unsupported(node));
        }
    }

    Ok(ArrowFunctionExpression {
        cv: None,
        params,
        body,
        is_async: false,
    })
}

/// `arrow_parameters = NAME | LPAREN [ formal_parameters ] RPAREN`.
///
/// A parenthesised list wraps a `formal_parameters` node; a single bare
/// identifier is just a `NAME` token (no wrapper); `()` has neither.
fn convert_arrow_parameters(node: &GrammarASTNode) -> Result<Vec<FunctionParam>, BridgeError> {
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "formal_parameters" {
                return convert_formal_parameters(n);
            }
        }
    }
    // No `formal_parameters` node → either `()` (no NAME tokens) or a bare
    // single `NAME` identifier. Any token that is not a paren is that name.
    let params = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value != "(" && t.value != ")" => {
                Some(FunctionParam::Identifier(Identifier { cv: None, name: t.value.clone() }))
            }
            _ => None,
        })
        .collect();
    Ok(params)
}

/// `concise_body = assignment_expression | LBRACE function_body RBRACE`.
///
/// In practice only the expression alternative is reachable today (see the
/// gap-156 note on [`convert_arrow_function`]); the `function_body` arm is
/// kept so the bridge is already correct once the grammar parses block
/// bodies.
fn convert_concise_body(node: &GrammarASTNode) -> Result<ArrowBody, BridgeError> {
    for n in node_children(node) {
        if n.rule_name == "function_body" {
            return Ok(ArrowBody::Block(convert_function_body(n)?));
        }
        return Ok(ArrowBody::Expression(Box::new(convert_expression(n)?)));
    }
    // Only brace tokens, no inner node → an empty block body `() => {}`.
    Ok(ArrowBody::Block(BlockStatement { cv: None, body: vec![] }))
}

/// Convert a `template_literal` node into an [`Expression::TemplateLiteral`]
/// (CLOC12.155).
///
/// **Scope: no-substitution templates only.** The grammar tokenises a
/// backtick template with no `${…}` inserts as a *single* `TEMPLATE_NO_SUB`
/// token whose value is the whole literal, backticks included
/// (`` `abc` `` → one token `"`abc`"`). Substitution templates
/// (`` `a${x}b` ``) do not parse at all today, so a `template_literal` node
/// that is anything other than exactly one such token is **declined**
/// (`UnsupportedSyntax` → the CLI falls back to WHITESPACE_ONLY, always
/// correct). When the grammar learns to parse `${…}` this converter grows a
/// multi-part branch; the AST node already models it.
///
/// The single backtick-delimited token becomes one tail [`TemplateElement`]:
/// we strip the leading and trailing `` ` `` to get the `raw` inner text.
/// `cooked` mirrors `raw` here — a no-substitution template with no illegal
/// escapes has a well-defined cooked value equal to its raw text for the
/// ASCII cases the SIMPLE pipeline sees (escape *processing* is a future
/// refinement; the emitter re-emits `raw` verbatim regardless, so this is
/// never a correctness hazard).
fn convert_template_literal(node: &GrammarASTNode) -> Result<TemplateLiteral, BridgeError> {
    // The node must be exactly one token and no child nodes — any child node
    // (a parsed `${…}` substitution, once the grammar supports it) is out of
    // scope for this slice.
    if !node_children(node).is_empty() {
        return Err(unsupported(node));
    }
    let tokens: Vec<&Token> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        })
        .collect();

    let [tok] = tokens.as_slice() else {
        // Zero or many tokens → not the single-token no-sub shape.
        return Err(unsupported(node));
    };

    // Guard the exact lexical shape: a `TEMPLATE_NO_SUB` token bounded by
    // backticks. Anything else (a stray substitution token, a malformed
    // literal) declines rather than risk mis-slicing.
    if tok.type_name.as_deref() != Some("TEMPLATE_NO_SUB") {
        return Err(unsupported(node));
    }
    let raw_full = &tok.value;
    let inner = raw_full
        .strip_prefix('`')
        .and_then(|s| s.strip_suffix('`'))
        .ok_or_else(|| unsupported(node))?
        .to_string();

    let element = TemplateElement {
        cv: tok.cv.clone(),
        raw: inner.clone(),
        cooked: Some(inner),
        tail: true,
    };
    Ok(TemplateLiteral {
        cv: tok.cv.clone(),
        quasis: vec![element],
        expressions: vec![],
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

        // A plain (non-generator, non-async) function in value position —
        // an IIFE `(function(){})()`, an assigned function
        // `x = function(){}`, a named recursive `function f(){…f()…}`, a
        // callback `arr.map(function(x){return x})`. Now that the typed AST
        // has `Expression::FunctionExpression` (CLOC12.149), convert it
        // instead of declining, so closurec optimises through it rather than
        // falling back to WHITESPACE_ONLY. (gap-153.)
        "function_expression" => {
            convert_function_expression(node).map(Expression::FunctionExpression)
        }

        // A generator function in value position (`x = function*(){…}`,
        // `(function*(){…})()`). Same converter as `function_expression` — the
        // `*` sets the `generator` flag so the emitter re-prints `function*`
        // (CLOC12.163 PR2, gap-164).
        "generator_expression" => {
            convert_function_expression(node).map(Expression::FunctionExpression)
        }

        // A `yield` / `yield* x` expression inside a generator body
        // (CLOC12.163 PR2, gap-164). Now that the typed AST has
        // `Expression::YieldExpression` (CLOC12.163 PR1) and the bridge
        // converts the enclosing generator function, convert the yield instead
        // of declining.
        "yield_expression" => convert_yield_expression(node),

        // Concise-body arrow function (CLOC12.152). `convert_arrow_function`
        // itself declines the ambiguous `() => {}` / object-body case and
        // (until the grammar parses them) never sees a block body.
        "arrow_function" => {
            convert_arrow_function(node).map(Expression::ArrowFunctionExpression)
        }

        // No-substitution template literal (CLOC12.155). `convert_template_literal`
        // declines any template that is not a single `TEMPLATE_NO_SUB` token —
        // i.e. anything with a `${…}` substitution, which the grammar does not
        // yet parse anyway (see the gap note on the converter).
        "template_literal" => {
            convert_template_literal(node).map(Expression::TemplateLiteral)
        }

        // The remaining function-valued and ES2015+ expression forms the
        // typed bridge does not yet represent. DECLINE GRACEFULLY
        // (`UnsupportedSyntax` → the CLI falls back to WHITESPACE_ONLY and
        // still emits valid JS). Generators/async carry evaluation semantics
        // the passes don't model yet; async arrows, classes, and *tagged*
        // template literals are separate future AST slices. (Plain
        // no-substitution templates are handled above by
        // `convert_template_literal`.)
        "async_arrow_function"
        | "await_expression"
        | "async_function_expression"
        | "async_generator_expression"
        | "class_expression"
        | "tagged_template_expression"
        | "new_target_expression"
        | "import_meta_expression" => Err(unsupported(node)),

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
    let nodes = node_children(node);
    if nodes.len() == 1 {
        convert_expression(nodes[0])
    } else if nodes.is_empty() {
        Err(internal(node, "expression: no children"))
    } else {
        // `a, b, c` — the comma operator (CLOC12.160 PR2). The `COMMA` tokens
        // are already dropped by `node_children`, so `nodes` is exactly the
        // `assignment_expression` operand list in source order. Convert each
        // into a `SequenceExpression` operand. (A single failed operand
        // propagates its error, dropping the whole file to WHITESPACE_ONLY.)
        let expressions = nodes
            .iter()
            .map(|n| convert_expression(n))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expression::SequenceExpression(SequenceExpression {
            cv: None,
            expressions,
        }))
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
    //
    // BUG HISTORY — *why this is not a simple child-count switch.* The
    // prefix operator (`!`, `-`, `typeof`, …) is a **token** child, and the
    // operand is an **AST-node** child. `node_children` deliberately drops
    // token children (it returns only `ASTNodeOrToken::Node`s), so BOTH
    // grammar alternatives expose *exactly one* AST child node:
    //
    //     postfix_expression           → children = [ Node(operand) ]
    //     "!" unary_expression         → children = [ Token("!"), Node(operand) ]
    //                                                              ^^^^^^^^^^^^^
    //                                     node_children() = [ Node(operand) ]  (len 1)
    //
    // The earlier `if node_children(node).len() == 1 { pass-through }`
    // therefore mis-classified *every* prefix-operator form as a
    // pass-through and silently returned the bare operand — `!a` emitted as
    // `a`, `-b` as `b`, `~c` as `c`, `typeof x` as `x`. That is a
    // **miscompile** (SIMPLE/ADVANCED), not a missed optimization:
    // WHITESPACE_ONLY kept the operator because it never runs the bridge.
    //
    // The correct discriminator is the *presence of a recognized prefix
    // operator token*, independent of how many AST child nodes there are.
    let op = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) => unary_operator_from_str(t.value.as_str()),
        _ => None,
    });
    let nodes = node_children(node);
    let arg_n = nodes
        .first()
        .ok_or_else(|| internal(node, "unary_expression: missing argument"))?;
    match op {
        // No RECOGNIZED *pure-unary* prefix operator. Either this is the genuine
        // `postfix_expression` pass-through alternative (just the operand), OR a
        // prefix `++`/`--` (an [`UpdateExpression`], a *separate* node from
        // `UnaryExpression` because `++`/`--` mutate their operand). The two are
        // distinguished by the presence of a `++`/`--` token: a bare operand
        // passes through; a prefix update becomes `UpdateExpression { prefix:
        // true }` over the converted operand. (CLOC12.158 PR2 — previously this
        // rejected the update as `UnsupportedSyntax` because the typed AST had
        // no `UpdateExpression`.)
        None => match update_operator_from_node(node) {
            Some(operator) => Ok(Expression::UpdateExpression(UpdateExpression {
                cv: None,
                operator,
                prefix: true,
                argument: Box::new(convert_expression(arg_n)?),
            })),
            None => convert_expression(arg_n),
        },
        Some(operator) => Ok(Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator,
            prefix: true,
            argument: Box::new(convert_expression(arg_n)?),
        })),
    }
}

/// Map a prefix-operator token's text to its [`UnaryOperator`]. Returns
/// `None` for any token that is not a unary prefix operator (the operand
/// of a `postfix_expression` pass-through, for instance), which lets the
/// caller use "did we find an operator?" as the alternative-discriminator.
fn unary_operator_from_str(s: &str) -> Option<UnaryOperator> {
    Some(match s {
        "-" => UnaryOperator::Negate,
        "+" => UnaryOperator::Plus,
        "!" => UnaryOperator::Not,
        "~" => UnaryOperator::BitNot,
        "typeof" => UnaryOperator::TypeOf,
        "void" => UnaryOperator::Void,
        "delete" => UnaryOperator::Delete,
        _ => return None,
    })
}

// -------------------------------------------------------------------------
// postfix_expression
// -------------------------------------------------------------------------

fn convert_postfix_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // postfix_expression = left_hand_side_expression [ PLUS_PLUS | MINUS_MINUS ]
    //
    // The optional `++`/`--` is a *token* child (dropped by `node_children`),
    // so both alternatives expose exactly one AST child: the operand. A present
    // `++`/`--` token becomes `UpdateExpression { prefix: false }` over that
    // operand; its absence is the plain pass-through. (CLOC12.158 PR2 —
    // previously the postfix form rejected as `UnsupportedSyntax`.)
    let nodes = node_children(node);
    let operand = match nodes.as_slice() {
        [operand] => *operand,
        _ => return Err(internal(node, "postfix_expression: unexpected shape")),
    };
    match update_operator_from_node(node) {
        Some(operator) => Ok(Expression::UpdateExpression(UpdateExpression {
            cv: None,
            operator,
            prefix: false,
            argument: Box::new(convert_expression(operand)?),
        })),
        None => convert_expression(operand),
    }
}

/// If `node` carries a `++` or `--` token child, return the matching
/// [`UpdateOperator`]; otherwise `None`. Used to distinguish an
/// `UpdateExpression` from a plain pass-through in both the prefix
/// (`unary_expression`) and postfix (`postfix_expression`) grammar forms.
fn update_operator_from_node(node: &GrammarASTNode) -> Option<UpdateOperator> {
    if has_token(node, "++") {
        Some(UpdateOperator::Increment)
    } else if has_token(node, "--") {
        Some(UpdateOperator::Decrement)
    } else {
        None
    }
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
    // `"new" new_expression` — the BARE `new X` form (no argument parens). It is
    // semantically identical to `new X()`, so we build a `NewExpression` with an
    // EMPTY argument list; the emitter prints the canonical `new X()`. (The
    // *argumented* `new X(args)` form is parsed as a `member_expression`, not a
    // `new_expression`, and is converted in `convert_member_expression`.)
    let callee_node =
        sole_node(node).ok_or_else(|| internal(node, "new_expression: expected callee after `new`"))?;
    let callee = convert_expression(callee_node)?;
    Ok(Expression::NewExpression(NewExpression {
        cv: None,
        callee: Box::new(callee),
        arguments: Vec::new(),
    }))
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

    // Optional chaining (`a?.b`, `f?.()`) is Phase 2. Decline so the CLI
    // falls back to WHITESPACE_ONLY rather than risk dropping the `?.`.
    if has_token(node, "?.") {
        return Err(unsupported(node));
    }

    // A `call_expression` node is a FLAT suffix chain: a base
    // (`member_expression` / `primary_expression`) followed by any number of
    // suffixes, in source order:
    //   - `arguments`   → a call            `base(args)`
    //   - `. NAME`      → dot member        `base.name`
    //   - `[ expr ]`    → computed member   `base[expr]`
    //
    // For example the parser yields, for
    //   `f().x`  : [member_expression(f), arguments(()), Token("."), Token("x")]
    //   `f()[k]` : [member_expression(f), arguments(()), "[", expression(k), "]"]
    //   `f()()`  : [member_expression(f), arguments(()), arguments(())]
    //
    // The earlier implementation inspected only the LAST child and dispatched
    // the whole node to a single handler, so any suffix the chosen handler
    // ignored was silently DROPPED — turning `f().x` into `f()` and `f()[k]`
    // into `f[k]` (real miscompiles; the call or the property vanished). We
    // instead fold EVERY suffix left-to-right onto the growing `base`,
    // mirroring `convert_member_expression`'s member walk with the
    // `arguments` (call) case added. This also subsumes the chained-call
    // `f()()` fold. Any token we don't recognise here is rejected
    // (fail-closed: an error feeds the WHITESPACE_ONLY fallback, never a
    // wrong program).
    let children = &node.children;
    let mut base = match children.first() {
        Some(ASTNodeOrToken::Node(n)) => convert_expression(n)?,
        _ => return Err(internal(node, "call_expression: expected a base expression")),
    };

    let mut i = 1;
    while i < children.len() {
        match &children[i] {
            // `(args)` — a call applied to the current base.
            ASTNodeOrToken::Node(n) if n.rule_name == "arguments" => {
                let args = convert_arguments(n)?;
                base = Expression::CallExpression(CallExpression {
                    cv: None,
                    callee: Box::new(base),
                    arguments: args,
                });
                i += 1;
            }
            // `.NAME` — dot (non-computed) member access.
            ASTNodeOrToken::Token(t) if t.value == "." => {
                let prop_name = match children.get(i + 1) {
                    Some(ASTNodeOrToken::Token(name)) => name.value.clone(),
                    _ => return Err(internal(node, "call_expression.dot: missing property name")),
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
            // `[expr]` — computed member access.
            ASTNodeOrToken::Token(t) if t.value == "[" => {
                // The key is the next Node child; skip to it.
                let key_node = children[i + 1..].iter().find_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                });
                let key = match key_node {
                    Some(n) => convert_expression(n)?,
                    None => return Err(internal(node, "call_expression.computed: missing key")),
                };
                base = Expression::MemberExpression(MemberExpression {
                    cv: None,
                    object: Box::new(base),
                    property: Box::new(key),
                    computed: true,
                });
                // Advance past `[ … ]` to the matching RBRACKET.
                i += 1;
                while i < children.len() {
                    let is_rbracket =
                        matches!(&children[i], ASTNodeOrToken::Token(t) if t.value == "]");
                    i += 1;
                    if is_rbracket {
                        break;
                    }
                }
            }
            // Anything else (tagged template, `new`/`super`, a stray token) is
            // not yet representable — fail closed rather than drop it.
            _ => return Err(unsupported(node)),
        }
    }

    Ok(base)
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
    //
    // A spread argument `f(...a)` parses to a `spread_element` node whose
    // children are `[ Token("..."), Node(assignment_expression) ]` — the
    // ELLIPSIS token sits directly under `spread_element` (confirmed by dumping
    // the parse tree), so `has_token(node, "...")` fires on exactly this shape.
    // Convert the inner expression and wrap it as `SpreadElement` (CLOC12.162
    // PR2, closes gap-163). `node_children` strips the ELLIPSIS token, leaving
    // the single assignment_expression Node.
    if has_token(node, "...") {
        let inner = node_children(node)
            .into_iter()
            .next()
            .ok_or_else(|| internal(node, "spread argument: missing expression"))?;
        return Ok(Expression::SpreadElement(SpreadElement {
            cv: None,
            argument: Box::new(convert_expression(inner)?),
        }));
    }
    // The parser collapses the single-alternative `argument` production, so the
    // node we receive here IS the `assignment_expression` itself. For an
    // assignment argument like `f(x = 1)` that node's children are
    //   [left_hand_side_expression(x), assignment_operator(=), assignment_expression(1)]
    // The previous implementation unwrapped to `node_children().next()` — the
    // FIRST child — which grabbed only the LHS `x` and silently dropped
    // `= 1`, miscompiling `f(x=1)` into `f(x)` (and `f(x+=1)` into `f(x)`,
    // `f(x=y=1)` into `f(x)`, …). We must convert the WHOLE node, so the
    // assignment is preserved by `convert_assignment_expression`.
    //
    // If a future grammar revision reintroduces an explicit `argument` wrapper
    // node (rather than the collapsed assignment_expression), unwrap it to its
    // sole child first; otherwise convert the node directly.
    let target = if node.rule_name == "argument" {
        node_children(node)
            .into_iter()
            .next()
            .ok_or_else(|| internal(node, "argument: missing expression"))?
    } else {
        node
    };
    convert_expression(target)
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

    // `new.target` — a meta-property, Phase 3. (The argumented `new X(args)`
    // form is converted below by the base-initialisation; only the
    // `"new" DOT "target"` meta-property is still declined here.)
    if has_token(node, "new")
        && node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "target"))
    {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "NewTarget".to_string(),
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

    // The base is either a primary_expression (`a`, `a.b`, `a[k]` …) or the
    // argumented `new` form `"new" member_expression arguments` (`new X(a,b)`).
    // We compute the starting `base` and the index `i` where the suffix chain
    // (`.NAME` / `[expr]`) begins, then the shared loop below folds any suffix
    // (so `new X().y` and `new X()[k]` fold correctly).
    let (mut base, mut i) = if matches!(
        children.first(),
        Some(ASTNodeOrToken::Token(t)) if t.value == "new"
    ) {
        // children: [Token("new"), Node(member_expression callee),
        //            Node(arguments), <optional .NAME / [expr] suffixes>].
        // The callee is the first Node child; the argument list is the first
        // `arguments`-rule Node. Construct the `NewExpression` and resume the
        // suffix walk just after the arguments node.
        let callee_node = children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            _ => None,
        });
        let args_idx = children.iter().position(
            |c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "arguments"),
        );
        match (callee_node, args_idx) {
            (Some(callee_node), Some(args_idx)) => {
                let callee = convert_expression(callee_node)?;
                let args_node = match &children[args_idx] {
                    ASTNodeOrToken::Node(n) => n,
                    _ => unreachable!("args_idx points at a Node by construction"),
                };
                let arguments = convert_arguments(args_node)?;
                let new_expr = Expression::NewExpression(NewExpression {
                    cv: None,
                    callee: Box::new(callee),
                    arguments,
                });
                (new_expr, args_idx + 1)
            }
            _ => {
                return Err(internal(
                    node,
                    "member_expression: `new` form missing callee or arguments",
                ))
            }
        }
    } else {
        // A plain primary base — always the first child Node.
        let base = match children.first() {
            Some(ASTNodeOrToken::Node(n)) => convert_expression(n)?,
            _ => return Err(internal(node, "member_expression: expected primary base")),
        };
        (base, 1)
    };
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
            // `` <base>`...` `` — a tagged template: the accumulated `base`
            // becomes the tag, and the `template_literal` becomes the quasi.
            // Reuse `convert_template_literal` (CLOC12.155) for the quasi — a
            // tagged template is structurally "an expression applied to a
            // template", so nothing new is parsed here. Wrapping continues the
            // suffix walk, so `` a`x`.length `` and `` a`x`() `` chain naturally.
            ASTNodeOrToken::Node(n) if n.rule_name == "template_literal" => {
                let quasi = convert_template_literal(n)?;
                base = Expression::TaggedTemplateExpression(TaggedTemplateExpression {
                    cv: None,
                    tag: Box::new(base),
                    quasi,
                });
                i += 1;
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
        "this" => return Ok(Expression::ThisExpression(ThisExpression { cv: t.cv.clone() })),
        "null" => return Ok(Expression::NullLiteral(NullLiteral { cv: t.cv.clone() })),
        "undefined" => return Ok(Expression::UndefinedLiteral(UndefinedLiteral { cv: t.cv.clone() })),
        "true" => return Ok(Expression::BooleanLiteral(BooleanLiteral { cv: t.cv.clone(), value: true })),
        "false" => return Ok(Expression::BooleanLiteral(BooleanLiteral { cv: t.cv.clone(), value: false })),
        _ => {}
    }

    // BIGINT: type_ == TokenType::Name but type_name == Some("BIGINT").
    if t.type_name.as_deref() == Some("BIGINT") {
        let raw = t.value.clone();
        let value = raw.trim_end_matches('n').to_string();
        return Ok(Expression::BigIntLiteral(BigIntLiteral { cv: t.cv.clone(), value, raw }));
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
                cv: t.cv.clone(),
                value: val,
                raw: t.value.clone(),
            }));
        }
        TokenType::String => {
            let raw = t.value.clone();
            let value = unquote_string(&raw);
            return Ok(Expression::StringLiteral(StringLiteral { cv: t.cv.clone(), value, raw }));
        }
        TokenType::Name => {
            // Plain identifier (variable name or non-keyword reference).
            return Ok(Expression::Identifier(Identifier { cv: t.cv.clone(), name: t.value.clone() }));
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
    Ok(Expression::Identifier(Identifier { cv: t.cv.clone(), name: t.value.clone() }))
}

// =========================================================================
// Literals
// =========================================================================

fn convert_array_literal(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // array_literal = LBRACKET [ element_list ] RBRACKET ;
    // element_list  = [ ELLIPSIS ] assignment_expression
    //                 { COMMA [ ELLIPSIS ] assignment_expression } [ COMMA ] ;
    //
    // ELISIONS (array holes). A comma that is NOT preceded by an element since
    // the previous comma (or the start) marks a HOLE: `[1,,3]` is `[1, <hole>, 3]`
    // of length 3, NOT `[1, 3]` of length 2. The distinction is observable —
    // `[1,,3].length === 3` and `1 in [1,,3] === false`, whereas `[1,3].length
    // === 2` and `1 in [1,3] === true` — so dropping a hole is a miscompile.
    //
    // The grammar keeps every comma as a Token child of `element_list`, but
    // `node_children` strips Token children, so the previous implementation
    // (which iterated `node_children`) never saw the commas and silently dropped
    // every hole. We therefore walk the RAW children of `element_list` here.
    //
    // `expect_element` is true at the start and immediately after each comma. A
    // comma seen while it is still true means the slot before that comma was
    // empty → push a hole. A lone *trailing* comma after an element is NOT a hole
    // (`[1,2,]` is length 2): the loop simply ends with `expect_element == true`
    // and pushes nothing more.
    let mut elements: Vec<Option<Expression>> = Vec::new();
    for n in node_children(node) {
        match n.rule_name.as_str() {
            "element_list" => {
                let mut expect_element = true;
                for c in &n.children {
                    match c {
                        ASTNodeOrToken::Token(t) if t.value == "," => {
                            if expect_element {
                                elements.push(None);
                            }
                            expect_element = true;
                        }
                        // A spread `[...x]` is not supported (Phase 2); the
                        // ELLIPSIS may appear either as a sibling token here or
                        // nested inside the element node (handled below).
                        ASTNodeOrToken::Token(t) if t.value == "..." => {
                            return Err(BridgeError::UnsupportedSyntax {
                                rule: "SpreadElement".to_string(),
                                location: loc(n),
                            });
                        }
                        ASTNodeOrToken::Token(_) => { /* stray token: ignore */ }
                        ASTNodeOrToken::Node(elem) => {
                            if has_token(elem, "...") {
                                // Spread element `[...x]` (CLOC12.162 PR2, closes
                                // gap-163). The `spread_element` node wraps the
                                // ELLIPSIS token and the inner
                                // assignment_expression; `node_children` strips
                                // the token, leaving the single expression Node.
                                let inner = node_children(elem)
                                    .into_iter()
                                    .next()
                                    .ok_or_else(|| {
                                        internal(elem, "spread element: missing expression")
                                    })?;
                                elements.push(Some(Expression::SpreadElement(SpreadElement {
                                    cv: None,
                                    argument: Box::new(convert_expression(inner)?),
                                })));
                                expect_element = false;
                                continue;
                            }
                            // `elem` is the `assignment_expression` for this
                            // slot. Convert it WHOLE: the previous code unwrapped
                            // to `node_children(elem).next()`, which for an
                            // assignment element grabbed only the LHS and dropped
                            // `= rhs`, miscompiling `[x=1]` into `[x]` (and
                            // `[a=1,b]` into `[a,b]`). `convert_expression`
                            // dispatches `assignment_expression` correctly for
                            // both the plain (`[x]`) and assignment (`[x=1]`)
                            // cases.
                            elements.push(Some(convert_expression(elem)?));
                            expect_element = false;
                        }
                    }
                }
            }
            // Single-element array with no `element_list` wrapper — no commas, so
            // no holes are possible.
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
    //
    // CRITICAL — the terminal token *kind* (NAME / NUMBER / STRING) lives in the
    // `t.type_` discriminant, NOT in `t.type_name`. `type_name` is `None` for
    // these ordinary terminals and is only populated for special tokens such as
    // BIGINT (see `convert_primary_token`'s contract note above). An earlier
    // version of this function matched on `t.type_name`, so a STRING or NUMBER
    // key NEVER matched and fell straight through to the NAME fallback below —
    // emitting EVERY quoted key as a BARE identifier built from the un-decoded
    // token text. Concretely that miscompiled:
    //
    //   {"a-b":1}        →  {a-b:1}        // SyntaxError: `-` not in an ident
    //   {"a b":1}        →  {a b:1}        // SyntaxError: space not in an ident
    //   {"x\ty":1}       →  {x\ty:1}       // SyntaxError: stray escape chars
    //   {"__proto__":1}  →  {__proto__:1}  // WORSE: own property silently became
    //                                      // the prototype setter — a DIFFERENT
    //                                      // object at runtime.
    //
    // We now switch on `t.type_`, exactly mirroring `convert_primary_token`, and
    // decode string keys through `unquote_string` so the key's `value` holds the
    // real property name. The quote-vs-bare *emission* choice is then made
    // soundly in the emitter (`emit_property_key`), which only drops the quotes
    // when the decoded name is a valid identifier (and never for `__proto__`).
    if has_token(node, "[") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "ComputedPropertyKey".to_string(),
            location: loc(node),
        });
    }
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.type_ {
                TokenType::String => {
                    let raw = t.value.clone();
                    let value = unquote_string(&raw);
                    return Ok(PropertyKey::StringLiteral(
                        coding_adventures_javascript_ast::expression::StringLiteral { cv: None, value, raw },
                    ));
                }
                TokenType::Number => {
                    let val: f64 = parse_js_number(&t.value).unwrap_or(0.0);
                    return Ok(PropertyKey::NumericLiteral(
                        coding_adventures_javascript_ast::expression::NumericLiteral {
                            cv: None,
                            value: val,
                            raw: t.value.clone(),
                        },
                    ));
                }
                // NAME / KEYWORD (reserved words ARE legal property names, e.g.
                // `{if: 1}`) and any other terminal: a bare identifier key.
                _ => {
                    return Ok(PropertyKey::Identifier(Identifier {
                        cv: None,
                        name: t.value.clone(),
                    }));
                }
            }
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

    // -----------------------------------------------------------------------
    // Template literals — no-substitution only (CLOC12.155)
    // -----------------------------------------------------------------------

    /// A no-substitution template `` `abc` `` bridges to a `TemplateLiteral`
    /// with a single tail quasi whose `raw` is the inner text (backticks
    /// stripped) and no `${…}` expressions.
    #[test]
    fn template_literal_no_substitution() {
        let p = bridge_ok("`abc`;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => match &es.expression {
                Expression::TemplateLiteral(t) => {
                    assert_eq!(t.expressions.len(), 0, "no-sub template has no inserts");
                    assert_eq!(t.quasis.len(), 1, "exactly one quasi");
                    assert_eq!(t.quasis[0].raw, "abc", "backticks stripped from raw");
                    assert_eq!(t.quasis[0].cooked.as_deref(), Some("abc"));
                    assert!(t.quasis[0].tail, "the sole quasi is the tail");
                }
                other => panic!("expected TemplateLiteral, got {other:?}"),
            },
            _ => panic!("expected ExpressionStatement"),
        }
    }

    /// An empty template `` `` `` bridges to a single empty-string quasi.
    #[test]
    fn template_literal_empty() {
        let p = bridge_ok("``;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => match &es.expression {
                Expression::TemplateLiteral(t) => {
                    assert_eq!(t.quasis.len(), 1);
                    assert_eq!(t.quasis[0].raw, "");
                    assert!(t.expressions.is_empty());
                }
                other => panic!("expected TemplateLiteral, got {other:?}"),
            },
            _ => panic!("expected ExpressionStatement"),
        }
    }

    /// A no-substitution template survives a `var` initializer end-to-end —
    /// the bridge no longer declines it (previously `UnsupportedSyntax`).
    #[test]
    fn template_literal_in_var_initializer() {
        let p = bridge_ok("var s = `hello`;");
        // Just assert the bridge accepted it and produced a TemplateLiteral
        // somewhere in the declaration initializer.
        let json = serde_json::to_string(&p).expect("serialize");
        assert!(json.contains("TemplateLiteral"), "initializer should bridge to a TemplateLiteral; got {json}");
        assert!(json.contains("hello"));
    }

    /// `` tag`abc` `` — a tagged template with an identifier tag: the bridge
    /// wraps the tag + template into a `TaggedTemplateExpression` (previously
    /// `UnsupportedSyntax`, gap-162).
    #[test]
    fn tagged_template_identifier_tag() {
        let p = bridge_ok("tag`abc`;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => match &es.expression {
                Expression::TaggedTemplateExpression(t) => {
                    match &*t.tag {
                        Expression::Identifier(id) => assert_eq!(id.name, "tag"),
                        other => panic!("expected identifier tag, got {other:?}"),
                    }
                    assert_eq!(t.quasi.quasis.len(), 1, "no-sub quasi");
                    assert_eq!(t.quasi.quasis[0].raw, "abc");
                    assert!(t.quasi.expressions.is_empty());
                }
                other => panic!("expected TaggedTemplateExpression, got {other:?}"),
            },
            _ => panic!("expected ExpressionStatement"),
        }
    }

    /// `` String.raw`abc` `` — a member-chain tag on a no-substitution template.
    /// The tag bridges to a `MemberExpression`; the quasi is a single tail
    /// element. (Substitution templates `` `a${x}b` `` do not parse in the
    /// grammar yet — see `convert_template_literal` — so the tagged form is
    /// exercised no-substitution here, matching the template bridge's scope.)
    #[test]
    fn tagged_template_member_tag() {
        let p = bridge_ok("String.raw`abc`;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => match &es.expression {
                Expression::TaggedTemplateExpression(t) => {
                    assert!(
                        matches!(&*t.tag, Expression::MemberExpression(_)),
                        "String.raw is a member-chain tag; got {:?}",
                        t.tag
                    );
                    assert_eq!(t.quasi.quasis.len(), 1, "single no-sub quasi");
                    assert_eq!(t.quasi.quasis[0].raw, "abc");
                    assert!(t.quasi.expressions.is_empty());
                }
                other => panic!("expected TaggedTemplateExpression, got {other:?}"),
            },
            _ => panic!("expected ExpressionStatement"),
        }
    }

    /// `` a`x`.length `` — a member access on a tagged template chains: the
    /// tagged template is the object of the outer member access.
    #[test]
    fn tagged_template_member_access_chains() {
        let p = bridge_ok("a`x`.length;");
        let json = serde_json::to_string(&p).expect("serialize");
        assert!(
            json.contains("TaggedTemplateExpression"),
            "member-on-tagged should keep the TaggedTemplateExpression; got {json}"
        );
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => match &es.expression {
                // Outer node is the `.length` member access whose object is the
                // tagged template.
                Expression::MemberExpression(m) => assert!(
                    matches!(&*m.object, Expression::TaggedTemplateExpression(_)),
                    "member object should be the tagged template; got {:?}",
                    m.object
                ),
                other => panic!("expected outer MemberExpression, got {other:?}"),
            },
            _ => panic!("expected ExpressionStatement"),
        }
    }

    /// A plain no-substitution template with NO tag still bridges to a bare
    /// `TemplateLiteral` (the tagged-template path does not capture untagged
    /// templates).
    #[test]
    fn untagged_template_still_bare() {
        let p = bridge_ok("`abc`;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => assert!(
                matches!(&es.expression, Expression::TemplateLiteral(_)),
                "untagged template must stay a bare TemplateLiteral; got {:?}",
                es.expression
            ),
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

    /// `this` — the reserved-word primary now bridges to `ThisExpression`
    /// (gap-166, CLOC12.165 PR2) rather than being declined as
    /// `UnsupportedSyntax`.
    #[test]
    fn this_expression() {
        let p = bridge_ok("this;");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es)
            )) => {
                assert!(matches!(&es.expression, Expression::ThisExpression(_)));
            }
            _ => panic!("expected ExpressionStatement"),
        }
    }

    /// `this.x` — `this` bridges as the object of a member access, proving it
    /// composes as a normal primary (not just as a bare statement).
    #[test]
    fn this_member_object() {
        let p = bridge_ok("this.x;");
        match only_expr(&p) {
            Expression::MemberExpression(m) => {
                assert!(matches!(&*m.object, Expression::ThisExpression(_)));
            }
            other => panic!("expected MemberExpression; got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Prefix unary expressions — regression for the operator-drop miscompile.
    //
    // The bridge used to discriminate the two `unary_expression` grammar
    // alternatives by counting AST child *nodes*, but the operator is a
    // *token* (filtered out by `node_children`), so both alternatives look
    // like a single child and every prefix operator was silently dropped
    // (`!a` bridged to bare `a`). These tests pin that each operator now
    // survives as a `UnaryExpression` with the correct `operator`.
    // -----------------------------------------------------------------------

    /// Pull the single expression out of a one-statement program.
    fn only_expr(p: &Program) -> &Expression {
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es),
            )) => &es.expression,
            _ => panic!("expected a single ExpressionStatement"),
        }
    }

    fn assert_prefix_unary(src: &str, expected_op: UnaryOperator) {
        let p = bridge_ok(src);
        match only_expr(&p) {
            Expression::UnaryExpression(u) => {
                assert_eq!(
                    u.operator, expected_op,
                    "operator mismatch for {src:?}: got {:?}",
                    u.operator
                );
                assert!(u.prefix, "prefix flag must be set for {src:?}");
            }
            other => panic!("expected UnaryExpression for {src:?}, got {other:?}"),
        }
    }

    #[test]
    fn prefix_not_survives_bridge() {
        assert_prefix_unary("!a;", UnaryOperator::Not);
    }

    #[test]
    fn prefix_negate_survives_bridge() {
        assert_prefix_unary("-a;", UnaryOperator::Negate);
    }

    #[test]
    fn prefix_plus_survives_bridge() {
        assert_prefix_unary("+a;", UnaryOperator::Plus);
    }

    #[test]
    fn prefix_bitnot_survives_bridge() {
        assert_prefix_unary("~a;", UnaryOperator::BitNot);
    }

    #[test]
    fn prefix_typeof_survives_bridge() {
        assert_prefix_unary("typeof a;", UnaryOperator::TypeOf);
    }

    #[test]
    fn prefix_void_survives_bridge() {
        assert_prefix_unary("void a;", UnaryOperator::Void);
    }

    #[test]
    fn prefix_delete_survives_bridge() {
        assert_prefix_unary("delete a.b;", UnaryOperator::Delete);
    }

    #[test]
    fn double_negation_nests_two_unaries() {
        // `!!a` must bridge to Unary(Not, Unary(Not, a)), not collapse.
        let p = bridge_ok("!!a;");
        match only_expr(&p) {
            Expression::UnaryExpression(outer) => {
                assert_eq!(outer.operator, UnaryOperator::Not);
                match outer.argument.as_ref() {
                    Expression::UnaryExpression(inner) => {
                        assert_eq!(inner.operator, UnaryOperator::Not);
                        assert!(matches!(inner.argument.as_ref(), Expression::Identifier(_)));
                    }
                    other => panic!("expected inner UnaryExpression, got {other:?}"),
                }
            }
            other => panic!("expected outer UnaryExpression, got {other:?}"),
        }
    }

    #[test]
    fn unary_pass_through_is_not_wrapped() {
        // The `postfix_expression` alternative (no operator token) must NOT
        // be wrapped in a UnaryExpression — `a;` stays a bare identifier.
        let p = bridge_ok("a;");
        assert!(
            matches!(only_expr(&p), Expression::Identifier(_)),
            "pass-through operand must not gain a spurious UnaryExpression"
        );
    }

    #[test]
    fn prefix_update_operators_convert_not_dropped() {
        // CLOC12.158 PR2: prefix `++`/`--` now convert to `UpdateExpression`
        // (they used to reject as `UnsupportedSyntax` while the typed AST had
        // no such node). They must NEVER silently drop the operator (`++a` →
        // `a` would be a miscompile) — assert a genuine UpdateExpression.
        assert!(matches!(sole_expr("++a;"), Expression::UpdateExpression(u) if u.prefix));
        assert!(matches!(sole_expr("--a;"), Expression::UpdateExpression(u) if u.prefix));
        assert!(matches!(sole_expr("++a.b;"), Expression::UpdateExpression(u) if u.prefix));
        // Genuine unary prefix operators are unaffected and stay UnaryExpression.
        assert!(matches!(sole_expr("-a;"), Expression::UnaryExpression(_)));
        assert!(matches!(sole_expr("!a;"), Expression::UnaryExpression(_)));
        assert!(matches!(sole_expr("typeof a;"), Expression::UnaryExpression(_)));
        // CRITICAL invariant: additive-with-unary-sign (`a + +b`, `a - -b`) are
        // SEPARATE `+`/`-` tokens, never a single `++`/`--`, so the shallow
        // `has_token(node, "++")` check must NOT false-positive them into an
        // update. They stay a binary `+`/`-` with a unary operand. Pin this
        // against any future change to `has_token`'s search depth.
        assert!(matches!(sole_expr("a + +b;"), Expression::BinaryExpression(_)));
        assert!(matches!(sole_expr("a - -b;"), Expression::BinaryExpression(_)));
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
    fn do_while_bridge_shape() {
        // CLOC20: `do { a(); } while (x)` now bridges to a DoWhileStatement
        // with the body (a BlockStatement) and the test (the identifier `x`).
        // Previously this returned UnsupportedSyntax → WHITESPACE_ONLY.
        let p = bridge_ok("do { a(); } while (x);");
        let t = match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::DoWhileStatement(d),
            )) => d.clone(),
            other => panic!("expected a DoWhileStatement, got {other:?}"),
        };
        // body is the `{ a(); }` block
        assert!(matches!(
            t.body.as_ref(),
            Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::BlockStatement(_)
            )
        ));
        // test is the identifier `x`
        assert!(
            matches!(&t.test, Expression::Identifier(id) if id.name == "x"),
            "expected test to be identifier `x`, got {:?}",
            t.test
        );
    }

    #[test]
    fn debugger_bridge_shape() {
        // CLOC21: `debugger;` now bridges to a DebuggerStatement (a bare
        // marker with no children). Previously this returned UnsupportedSyntax
        // → WHITESPACE_ONLY.
        let p = bridge_ok("debugger;");
        assert!(
            matches!(
                &p.body[0],
                ProgramItem::Statement(Statement::Tagged(
                    coding_adventures_javascript_ast::statement::TaggedStatement::DebuggerStatement(
                        _
                    )
                ))
            ),
            "expected a DebuggerStatement, got {:?}",
            p.body[0]
        );
    }

    /// Pull the single `ForInStatement` out of a one-statement program.
    fn bridge_for_in(src: &str) -> ForInStatement {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ForInStatement(f),
            )) => f.clone(),
            other => panic!("expected a ForInStatement, got {other:?}"),
        }
    }

    #[test]
    fn for_in_var_bridge_shape() {
        // CLOC22: `for (var k in obj) { f(k); }` bridges to a ForInStatement
        // whose left is a `var` declaration of a single binding `k`, right is
        // the identifier `obj`, and body is the block.
        let f = bridge_for_in("for (var k in obj) { f(k); }");
        match &f.left {
            ForInit::VariableDeclaration(v) => {
                assert_eq!(v.kind, VarKind::Var);
                assert_eq!(v.declarations.len(), 1);
                assert!(v.declarations[0].init.is_none(), "for-in binding has no init");
            }
            other => panic!("expected a var declaration left, got {other:?}"),
        }
        assert!(
            matches!(&f.right, Expression::Identifier(id) if id.name == "obj"),
            "expected right = `obj`, got {:?}",
            f.right
        );
        assert!(matches!(
            f.body.as_ref(),
            Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::BlockStatement(_)
            )
        ));
    }

    #[test]
    fn for_in_lexical_binding_left_bridge_shape() {
        // `for (let k in o)` and `for (const k in o)` bridge to a
        // ForInStatement whose left is a Let/Const single-binding declaration.
        for (src, want) in [
            ("for (let k in o) { f(); }", VarKind::Let),
            ("for (const k in o) { f(); }", VarKind::Const),
        ] {
            let f = bridge_for_in(src);
            match &f.left {
                ForInit::VariableDeclaration(v) => {
                    assert_eq!(v.kind, want, "kind mismatch for {src}");
                    assert_eq!(v.declarations.len(), 1);
                    assert!(v.declarations[0].init.is_none());
                }
                other => panic!("expected a lexical declaration left for {src}, got {other:?}"),
            }
        }
    }

    #[test]
    fn for_in_expression_left_bridge_shape() {
        // `for (k in obj) { f(k); }` — an existing assignment target as the
        // left bridges to `ForInit::Expression`.
        let f = bridge_for_in("for (k in obj) { f(k); }");
        assert!(
            matches!(&f.left, ForInit::Expression(Expression::Identifier(id)) if id.name == "k"),
            "expected expression left `k`, got {:?}",
            f.left
        );
    }

    #[test]
    fn for_in_destructuring_or_unrepresentable_left_does_not_hard_error() {
        // A destructuring for-in left (`for (var [a] in o)`) — or any other
        // left shape the declarator converter can't represent — must DECLINE
        // gracefully (parse-error or UnsupportedSyntax → WHITESPACE_ONLY at the
        // CLI), never hard-error or mis-bind. We assert only that bridging does
        // not panic and that if a ForInStatement IS produced its left is a
        // plain (non-destructuring) binding.
        for src in [
            "for (var [a] in o) { f(); }",
            "for (let {a} in o) { f(); }",
        ] {
            let Ok(node) = parse_javascript_typed(src, DEFAULT_ES_VERSION) else {
                continue; // parse declined — sound fallback
            };
            let _ = grammar_to_program(&node, DEFAULT_ES_VERSION); // must not panic
        }
    }

    #[test]
    fn destructuring_declarations_decline_gracefully_not_hard_error() {
        // `var [a,b]=c;`, `let {p,q}=o;`, `const [x]=y;` — destructuring binding
        // patterns are Phase 2. The bridge must decline with `UnsupportedSyntax`
        // (so the CLI falls back to WHITESPACE_ONLY and still emits valid JS),
        // NOT raise an `Internal` error — which the CLI treats as a hard failure
        // (`exit 2`, no output). Regression: the binding-pattern check sat after
        // the NAME-token unwrap, so the unwrap fired `internal("missing name")`
        // first and `var [a,b]=c;` failed to compile at SIMPLE/ADVANCED.
        for src in ["var [a,b]=c;", "let {p,q}=o;", "const [x]=y;"] {
            match bridge(src) {
                Err(BridgeError::UnsupportedSyntax { .. }) => {} // graceful decline
                other => panic!("expected UnsupportedSyntax for {src:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn function_expressions_convert_to_typed_nodes() {
        // CLOC12.149 / gap-153: a `function` expression in value position —
        // an IIFE, an assigned function, a callback argument, a named
        // recursive one — now converts to `Expression::FunctionExpression`
        // instead of declining. (It used to fall back to WHITESPACE_ONLY.)
        for src in [
            "(function(){})();",
            "x=function(){};",
            "f(function(){});",
            "var g=function h(){};",
        ] {
            bridge(src).unwrap_or_else(|e| panic!("expected {src:?} to bridge, got {e}"));
        }
    }

    /// Walk to the `FunctionExpression` inside a `var g = <fe>;` program. A
    /// `var` at statement position wraps as
    /// `ProgramItem::Statement(Statement::Declaration(..))`.
    fn bridge_var_init_fn_expr(src: &str) -> FunctionExpression {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(vd))) => {
                match vd.declarations[0].init.as_ref().expect("init") {
                    Expression::FunctionExpression(fe) => fe.clone(),
                    other => panic!("expected FunctionExpression init, got {other:?}"),
                }
            }
            other => panic!("expected a VariableDeclaration statement, got {other:?}"),
        }
    }

    #[test]
    fn named_function_expression_carries_body_local_name() {
        // `var g = function h (a) { return a; }` — the expression's own name
        // is `h` (body-local), distinct from the outer binding `g`; one param
        // `a`; a return body.
        let fe = bridge_var_init_fn_expr("var g=function h(a){return a;};");
        assert_eq!(fe.id.as_ref().map(|i| i.name.as_str()), Some("h"));
        assert_eq!(fe.params.len(), 1);
        assert!(!fe.generator && !fe.is_async);
        assert_eq!(fe.body.body.len(), 1, "one return statement");
    }

    #[test]
    fn anonymous_function_expression_has_no_id() {
        // `var g = function () {}` — anonymous, so `id` is `None`.
        let fe = bridge_var_init_fn_expr("var g=function(){};");
        assert!(fe.id.is_none(), "anonymous fn-expr must have no id");
        assert!(fe.params.is_empty());
        assert!(fe.body.body.is_empty());
    }

    #[test]
    fn iife_callee_is_a_function_expression() {
        // `(function(){})()` — the CallExpression's callee is a
        // FunctionExpression (the IIFE shape closurec must optimise/emit).
        let p = bridge_ok("(function(){})();");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es),
            )) => match &es.expression {
                Expression::CallExpression(call) => {
                    assert!(
                        matches!(*call.callee, Expression::FunctionExpression(_)),
                        "IIFE callee should be a FunctionExpression, got {:?}",
                        call.callee
                    );
                }
                other => panic!("expected a CallExpression, got {other:?}"),
            },
            other => panic!("expected an ExpressionStatement, got {other:?}"),
        }
    }

    // ---- ArrowFunctionExpression (CLOC12.152 bridge enable) ------

    /// Pull the arrow-function initialiser out of `var f = <arrow>;`.
    fn bridge_var_init_arrow(src: &str) -> ArrowFunctionExpression {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(vd))) => {
                match vd.declarations[0].init.as_ref().expect("init") {
                    Expression::ArrowFunctionExpression(a) => a.clone(),
                    other => panic!("expected ArrowFunctionExpression init, got {other:?}"),
                }
            }
            other => panic!("expected a VariableDeclaration statement, got {other:?}"),
        }
    }

    #[test]
    fn arrow_single_param_concise_body_converts() {
        // `var f = x => x + 1` — one param `x`, a concise (expression) body.
        let a = bridge_var_init_arrow("var f=x=>x+1;");
        assert!(!a.is_async);
        assert_eq!(a.params.len(), 1);
        let FunctionParam::Identifier(p) = &a.params[0];
        assert_eq!(p.name, "x");
        assert!(
            matches!(a.body, ArrowBody::Expression(_)),
            "concise body must be ArrowBody::Expression, got {:?}",
            a.body
        );
    }

    #[test]
    fn arrow_multi_param_and_empty_param_convert() {
        let a = bridge_var_init_arrow("var g=(a,b)=>a;");
        assert_eq!(a.params.len(), 2);
        let empty = bridge_var_init_arrow("var h=()=>1;");
        assert!(empty.params.is_empty(), "`()` yields zero params");
        assert!(matches!(empty.body, ArrowBody::Expression(_)));
    }

    #[test]
    fn arrow_callback_argument_converts() {
        // `arr.map(x => x)` — the callback arg is an ArrowFunctionExpression.
        let p = bridge_ok("arr.map(x=>x);");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es),
            )) => match &es.expression {
                Expression::CallExpression(call) => assert!(
                    matches!(&call.arguments[0], Expression::ArrowFunctionExpression(_)),
                    "callback arg should be an arrow, got {:?}",
                    call.arguments[0]
                ),
                other => panic!("expected CallExpression, got {other:?}"),
            },
            other => panic!("expected ExpressionStatement, got {other:?}"),
        }
    }

    #[test]
    fn arrow_object_concise_body_is_declined() {
        // `() => ({})` / `() => {}` are indistinguishable in the current
        // grammar (both parse as an object-literal concise body), so the
        // bridge DECLINES them (UnsupportedSyntax) rather than risk the
        // empty-block-vs-object miscompile. A declined program surfaces as a
        // bridge error → the CLI's whitespace-only passthrough.
        assert!(
            grammar_to_program(
                &crate::parse_javascript("var f=()=>({a:1});", "es2025").expect("parse"),
                DEFAULT_ES_VERSION,
            )
            .is_err(),
            "object-body arrow must decline to avoid the () => {{}} ambiguity"
        );
    }

    #[test]
    fn async_arrow_is_still_declined() {
        // Async arrows parse under `async_arrow_function` and remain declined
        // (safe whitespace-only passthrough) until the async model lands.
        assert!(
            grammar_to_program(
                &crate::parse_javascript("var f=async x=>x;", "es2025").expect("parse"),
                DEFAULT_ES_VERSION,
            )
            .is_err(),
            "async arrow should still decline"
        );
    }

    /// Pull the single `ForOfStatement` out of a one-statement program.
    fn bridge_for_of(src: &str) -> ForOfStatement {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ForOfStatement(f),
            )) => f.clone(),
            other => panic!("expected a ForOfStatement, got {other:?}"),
        }
    }

    #[test]
    fn for_of_bridge_shapes() {
        // CLOC23: all the binding-kind left forms plus the expression left.
        for (src, want) in [
            ("for (var v of it) { f(v); }", VarKind::Var),
            ("for (let v of it) { f(v); }", VarKind::Let),
            ("for (const v of it) { f(v); }", VarKind::Const),
        ] {
            let f = bridge_for_of(src);
            match &f.left {
                ForInit::VariableDeclaration(vd) => {
                    assert_eq!(vd.kind, want, "kind mismatch for {src}");
                    assert_eq!(vd.declarations.len(), 1);
                    assert!(vd.declarations[0].init.is_none());
                }
                other => panic!("expected a declaration left for {src}, got {other:?}"),
            }
            assert!(
                matches!(&f.right, Expression::Identifier(id) if id.name == "it"),
                "expected right = `it` for {src}, got {:?}",
                f.right
            );
        }
        // Expression left.
        let f = bridge_for_of("for (v of it) { f(v); }");
        assert!(
            matches!(&f.left, ForInit::Expression(Expression::Identifier(id)) if id.name == "v"),
            "expected expression left `v`, got {:?}",
            f.left
        );
    }

    #[test]
    fn for_of_using_or_destructuring_left_does_not_hard_error() {
        // A `using` binding (`for (using v of it)`) and destructuring lefts are
        // not modelled; they must DECLINE gracefully (parse-error or
        // UnsupportedSyntax → WHITESPACE_ONLY), never hard-error or mis-bind.
        for src in [
            "for (using v of it) { f(); }",
            "for (var [a] of it) { f(); }",
            "for (const {a} of it) { f(); }",
        ] {
            let Ok(node) = parse_javascript_typed(src, DEFAULT_ES_VERSION) else {
                continue; // parse declined — sound fallback
            };
            // Must not panic. If a ForOfStatement is produced at all, its left
            // must be a plain (non-destructuring) binding.
            if let Ok(p) = grammar_to_program(&node, DEFAULT_ES_VERSION) {
                if let Some(ProgramItem::Statement(Statement::Tagged(
                    coding_adventures_javascript_ast::statement::TaggedStatement::ForOfStatement(f),
                ))) = p.body.first()
                {
                    if let ForInit::VariableDeclaration(vd) = &f.left {
                        for d in &vd.declarations {
                            let coding_adventures_javascript_ast::BindingTarget::Identifier(_) =
                                &d.id;
                        }
                    }
                }
            }
        }
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
    fn chained_call_expression() {
        // `f()()` — the callee of the OUTER call is itself the call `f()`.
        // Regression: the parser flattens chained left-recursive calls into a
        // single call_expression node `[member_expression(f), arguments, arguments]`,
        // and the bridge previously tried to convert the inner `arguments` node
        // as an expression, raising "unknown expression rule 'arguments'".
        let outer = match first_expr(&bridge_ok("f()();")) {
            Expression::CallExpression(c) => c.clone(),
            other => panic!("expected CallExpression, got {other:?}"),
        };
        assert_eq!(outer.arguments.len(), 0);
        match &*outer.callee {
            Expression::CallExpression(inner) => {
                assert_eq!(inner.arguments.len(), 0);
                assert!(matches!(&*inner.callee, Expression::Identifier(id) if id.name == "f"));
            }
            other => panic!("expected inner CallExpression, got {other:?}"),
        }
    }

    #[test]
    fn triple_chained_call_with_args() {
        // `f(1)(2)(3)` folds left-to-right: ((f(1))(2))(3). Verify the nesting
        // and that each call site keeps its own single argument.
        let c3 = match first_expr(&bridge_ok("f(1)(2)(3);")) {
            Expression::CallExpression(c) => c.clone(),
            other => panic!("expected CallExpression, got {other:?}"),
        };
        assert_eq!(c3.arguments.len(), 1); // (3)
        let c2 = match &*c3.callee {
            Expression::CallExpression(c) => c.clone(),
            other => panic!("expected CallExpression, got {other:?}"),
        };
        assert_eq!(c2.arguments.len(), 1); // (2)
        let c1 = match &*c2.callee {
            Expression::CallExpression(c) => c.clone(),
            other => panic!("expected CallExpression, got {other:?}"),
        };
        assert_eq!(c1.arguments.len(), 1); // (1)
        assert!(matches!(&*c1.callee, Expression::Identifier(id) if id.name == "f"));
    }

    #[test]
    fn dot_member_on_call_result() {
        // `f().x` — a dot member access on a CALL result. Regression: the
        // bridge dispatched the whole call_expression node on its last child
        // and dropped the trailing `.x`, miscompiling `f().x` into `f()`.
        // The property read must survive: MemberExpression{ object: f(), .x }.
        let m = match first_expr(&bridge_ok("f().x;")) {
            Expression::MemberExpression(m) => m.clone(),
            other => panic!("expected MemberExpression, got {other:?}"),
        };
        assert!(!m.computed);
        assert!(matches!(&*m.property, Expression::Identifier(id) if id.name == "x"));
        match &*m.object {
            Expression::CallExpression(c) => {
                assert_eq!(c.arguments.len(), 0);
                assert!(matches!(&*c.callee, Expression::Identifier(id) if id.name == "f"));
            }
            other => panic!("expected the object to be the call `f()`, got {other:?}"),
        }
    }

    #[test]
    fn computed_member_on_call_result() {
        // `f()[k]` — a computed member access on a CALL result. Regression:
        // the bridge took the FIRST child as the base and skipped the
        // `arguments` node, miscompiling `f()[k]` into `f[k]` (the call
        // vanished). The object must be the call `f()`, key `k`, computed.
        let m = match first_expr(&bridge_ok("f()[k];")) {
            Expression::MemberExpression(m) => m.clone(),
            other => panic!("expected MemberExpression, got {other:?}"),
        };
        assert!(m.computed);
        assert!(matches!(&*m.property, Expression::Identifier(id) if id.name == "k"));
        match &*m.object {
            Expression::CallExpression(c) => {
                assert_eq!(c.arguments.len(), 0);
                assert!(matches!(&*c.callee, Expression::Identifier(id) if id.name == "f"));
            }
            other => panic!("expected the object to be the call `f()`, got {other:?}"),
        }
    }

    #[test]
    fn call_member_call_mixed_chain() {
        // `f().x()` — call, then dot member, then call. The whole flat suffix
        // chain must fold: CallExpression{ callee: (f()).x, args: [] }. Before
        // the fix this bridged to `unsupported` (and fell back to passthrough);
        // now it must produce the correctly nested AST.
        let outer = match first_expr(&bridge_ok("f().x();")) {
            Expression::CallExpression(c) => c.clone(),
            other => panic!("expected outer CallExpression, got {other:?}"),
        };
        assert_eq!(outer.arguments.len(), 0);
        let member = match &*outer.callee {
            Expression::MemberExpression(m) => m.clone(),
            other => panic!("expected callee to be member `f().x`, got {other:?}"),
        };
        assert!(!member.computed);
        assert!(matches!(&*member.property, Expression::Identifier(id) if id.name == "x"));
        match &*member.object {
            Expression::CallExpression(c) => {
                assert!(matches!(&*c.callee, Expression::Identifier(id) if id.name == "f"));
            }
            other => panic!("expected member object to be `f()`, got {other:?}"),
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
    // try / catch / finally (CLOC19)
    //
    // The bridge maps the grammar's `try_statement` into the ESTree-shaped
    // `TryStatement { block, handler, finalizer }`. Before CLOC19 the
    // try_statement node landed in the unsupported arm, which raised
    // `UnsupportedSyntax` and made closurec fall back to WHITESPACE_ONLY.
    // These tests pin the structural conversion directly.
    // -----------------------------------------------------------------------

    /// Pull the single `TryStatement` out of a one-statement program.
    fn bridge_try(src: &str) -> TryStatement {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::TryStatement(t),
            )) => t.clone(),
            other => panic!("expected a TryStatement, got {other:?}"),
        }
    }

    #[test]
    fn try_catch_bridge_shape() {
        // `try { a(); } catch (e) { b(); }` — protected block + named
        // handler, no finalizer.
        let t = bridge_try("try { a(); } catch (e) { b(); }");
        assert_eq!(t.block.body.len(), 1, "protected block has one statement");
        let handler = t.handler.expect("handler present");
        assert_eq!(
            handler.param.as_ref().map(|p| p.name.as_str()),
            Some("e"),
            "catch binding is `e`",
        );
        assert_eq!(handler.body.body.len(), 1, "catch body has one statement");
        assert!(t.finalizer.is_none(), "no finally clause");
    }

    #[test]
    fn try_catch_finally_bridge_shape() {
        // All three clauses present.
        let t = bridge_try("try { a(); } catch (e) { b(); } finally { c(); }");
        assert!(t.handler.is_some(), "handler present");
        let fin = t.finalizer.expect("finalizer present");
        assert_eq!(fin.body.len(), 1, "finally block has one statement");
    }

    #[test]
    fn try_optional_catch_binding_bridge_shape() {
        // ES2019 `catch { … }` — handler present, but `param` is None.
        let t = bridge_try("try { a(); } catch { b(); }");
        let handler = t.handler.expect("handler present");
        assert!(
            handler.param.is_none(),
            "optional-catch-binding has no param",
        );
    }

    #[test]
    fn try_finally_without_catch_bridge_shape() {
        // `try { … } finally { … }` — no handler at all.
        let t = bridge_try("try { a(); } finally { c(); }");
        assert!(t.handler.is_none(), "no catch handler");
        assert!(t.finalizer.is_some(), "finally present");
    }

    #[test]
    fn try_destructuring_catch_param_does_not_misbind() {
        // A destructuring catch param (`catch ({ message })`) is not
        // representable yet. The grammar restricts the catch binding to a
        // simple NAME, so this either fails to parse outright or fails to
        // bridge — both of which make the CLI fall back to WHITESPACE_ONLY,
        // which is sound. What must NEVER happen is a TryStatement whose
        // handler param is a fabricated simple identifier (silently
        // dropping the destructuring), so we assert that explicitly.
        let src = "try { a(); } catch ({ message }) { b(); }";
        let Ok(node) = parse_javascript_typed(src, DEFAULT_ES_VERSION) else {
            // Parse declined — sound fallback, nothing more to check.
            return;
        };
        let Ok(p) = grammar_to_program(&node, DEFAULT_ES_VERSION) else {
            // Bridge declined — sound fallback, nothing more to check.
            return;
        };
        if let Some(ProgramItem::Statement(Statement::Tagged(
            coding_adventures_javascript_ast::statement::TaggedStatement::TryStatement(t),
        ))) = p.body.first()
        {
            assert!(
                t.handler
                    .as_ref()
                    .map(|h| h.param.is_none())
                    .unwrap_or(true),
                "a destructuring catch param must not be lowered to a simple \
                 identifier — that would silently mis-bind the caught value",
            );
        }
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
    fn assignment_expression_as_call_argument_is_not_dropped() {
        // Regression: `convert_argument` unwrapped to the FIRST child of the
        // (collapsed) `assignment_expression` argument node, grabbing only the
        // LHS and dropping `= rhs` — miscompiling `f(x=1)` into `f(x)`. The
        // argument must bridge to a whole AssignmentExpression.
        let call = match first_expr(&bridge_ok("f(x=1);")) {
            Expression::CallExpression(c) => c.clone(),
            other => panic!("expected CallExpression, got {other:?}"),
        };
        assert_eq!(call.arguments.len(), 1);
        match &call.arguments[0] {
            Expression::AssignmentExpression(a) => {
                assert!(matches!(a.operator, AssignmentOperator::Eq));
                assert!(matches!(&a.left, AssignmentTarget::Identifier(id) if id.name == "x"));
            }
            other => panic!("expected the argument to be an assignment `x=1`, got {other:?}"),
        }
    }

    #[test]
    fn compound_and_chained_assignment_arguments_survive() {
        // `f(x+=1)` must keep the compound operator; `f(x=y=1)` must keep the
        // nested assignment. Both previously collapsed to `f(x)`.
        match first_expr(&bridge_ok("f(x+=1);")) {
            Expression::CallExpression(c) => match &c.arguments[0] {
                Expression::AssignmentExpression(a) => {
                    assert!(matches!(a.operator, AssignmentOperator::AddEq));
                }
                other => panic!("expected `x+=1` assignment arg, got {other:?}"),
            },
            other => panic!("expected CallExpression, got {other:?}"),
        }
        match first_expr(&bridge_ok("f(x=y=1);")) {
            Expression::CallExpression(c) => match &c.arguments[0] {
                // outer `x = (y = 1)` — the right side is itself an assignment.
                Expression::AssignmentExpression(a) => {
                    assert!(matches!(&*a.right, Expression::AssignmentExpression(_)));
                }
                other => panic!("expected nested assignment arg, got {other:?}"),
            },
            other => panic!("expected CallExpression, got {other:?}"),
        }
    }

    #[test]
    fn assignment_expression_as_array_element_is_not_dropped() {
        // Regression: `convert_array_literal` unwrapped each element to its
        // first child, dropping `= rhs` — `[x=1]` became `[x]`. The element
        // must bridge to a whole AssignmentExpression; a following plain
        // element (`[a=1,b]`) must still bridge normally.
        let arr = match first_expr(&bridge_ok("[a=1,b];")) {
            Expression::ArrayExpression(a) => a.clone(),
            other => panic!("expected ArrayExpression, got {other:?}"),
        };
        assert_eq!(arr.elements.len(), 2);
        match &arr.elements[0] {
            Some(Expression::AssignmentExpression(a)) => {
                assert!(matches!(a.operator, AssignmentOperator::Eq));
                assert!(matches!(&a.left, AssignmentTarget::Identifier(id) if id.name == "a"));
            }
            other => panic!("expected element 0 to be assignment `a=1`, got {other:?}"),
        }
        assert!(matches!(&arr.elements[1], Some(Expression::Identifier(id)) if id.name == "b"));
    }

    /// The first property's key of an object-literal expression statement.
    fn first_object_key(src: &str) -> PropertyKey {
        match first_expr(&bridge_ok(src)) {
            Expression::ObjectExpression(o) => o.properties[0].key.clone(),
            other => panic!("expected ObjectExpression, got {other:?}"),
        }
    }

    /// The array-literal expression's elements rendered as a compact
    /// hole-pattern string: `e` for a present element, `_` for a hole.
    fn array_hole_pattern(src: &str) -> String {
        match first_expr(&bridge_ok(src)) {
            Expression::ArrayExpression(a) => a
                .elements
                .iter()
                .map(|e| if e.is_some() { 'e' } else { '_' })
                .collect(),
            other => panic!("expected ArrayExpression, got {other:?}"),
        }
    }

    #[test]
    fn array_elisions_become_holes_not_dropped() {
        // Regression: `convert_array_literal` iterated `node_children`, which
        // strips the COMMA tokens, so every elision (hole) was silently dropped —
        // `[1,,3]` became a length-2 dense array. Holes must survive as `None`.
        assert_eq!(array_hole_pattern("[1,,3];"), "e_e"); // length 3, hole at 1
        assert_eq!(array_hole_pattern("[,,];"), "__"); // two leading holes, length 2
        assert_eq!(array_hole_pattern("[1,,];"), "e_"); // trailing hole, length 2
        assert_eq!(array_hole_pattern("[,1];"), "_e"); // leading hole, length 2
        assert_eq!(array_hole_pattern("[1,,,2];"), "e__e"); // two holes, length 4
        assert_eq!(array_hole_pattern("[,];"), "_"); // single hole, length 1
        // A *trailing comma* after an element is not a hole.
        assert_eq!(array_hole_pattern("[1,2,3,];"), "eee"); // length 3
        assert_eq!(array_hole_pattern("[1,2,3];"), "eee");
        assert_eq!(array_hole_pattern("[1];"), "e");
        assert_eq!(array_hole_pattern("[];"), "");
    }

    #[test]
    fn object_string_key_is_string_literal_not_identifier() {
        // Regression: STRING/NUMBER token kinds live in `t.type_`, not
        // `t.type_name`. The old code matched `type_name` and so turned EVERY
        // quoted key into a bare `Identifier` built from un-decoded text. A
        // quoted key must parse to a decoded `StringLiteral`.
        match first_object_key("({\"abc\": 1});") {
            PropertyKey::StringLiteral(s) => assert_eq!(s.value, "abc"),
            other => panic!("expected StringLiteral key, got {other:?}"),
        }
        // A non-identifier key (would be invalid JS if emitted bare).
        match first_object_key("({\"a-b\": 1});") {
            PropertyKey::StringLiteral(s) => assert_eq!(s.value, "a-b"),
            other => panic!("expected StringLiteral key, got {other:?}"),
        }
        // Escapes are DECODED into the value (`\t` → a real tab), so downstream
        // emission/folding sees the true property name.
        match first_object_key("({\"x\\ty\": 1});") {
            PropertyKey::StringLiteral(s) => assert_eq!(s.value, "x\ty"),
            other => panic!("expected StringLiteral key, got {other:?}"),
        }
        // `__proto__` as a quoted key is an ordinary own property — it must stay
        // a StringLiteral so the emitter keeps it quoted (not the proto setter).
        match first_object_key("({\"__proto__\": 1});") {
            PropertyKey::StringLiteral(s) => assert_eq!(s.value, "__proto__"),
            other => panic!("expected StringLiteral key, got {other:?}"),
        }
    }

    #[test]
    fn object_numeric_key_is_numeric_literal() {
        match first_object_key("({1: 2});") {
            PropertyKey::NumericLiteral(n) => assert_eq!(n.value, 1.0),
            other => panic!("expected NumericLiteral key, got {other:?}"),
        }
    }

    #[test]
    fn object_bare_name_key_is_identifier() {
        // A genuine bare identifier key stays an Identifier (incl. reserved
        // words, which are legal property names).
        match first_object_key("({abc: 1});") {
            PropertyKey::Identifier(i) => assert_eq!(i.name, "abc"),
            other => panic!("expected Identifier key, got {other:?}"),
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

    // -----------------------------------------------------------------------
    // Update expressions — `++` / `--` (CLOC12.158 PR2, closes gap-159)
    // -----------------------------------------------------------------------

    /// Extract the sole expression-statement's expression from a program.
    fn sole_expr(src: &str) -> Expression {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ExpressionStatement(es),
            )) => es.expression.clone(),
            other => panic!("expected ExpressionStatement, got {other:?}"),
        }
    }

    /// `a++` bridges to a postfix `UpdateExpression` (Increment) over `a`.
    #[test]
    fn postfix_increment() {
        match sole_expr("a++;") {
            Expression::UpdateExpression(u) => {
                assert_eq!(u.operator, UpdateOperator::Increment);
                assert!(!u.prefix, "a++ is postfix");
                assert!(matches!(&*u.argument, Expression::Identifier(i) if i.name == "a"));
            }
            other => panic!("expected UpdateExpression, got {other:?}"),
        }
    }

    /// `a--` bridges to a postfix `UpdateExpression` (Decrement).
    #[test]
    fn postfix_decrement() {
        match sole_expr("a--;") {
            Expression::UpdateExpression(u) => {
                assert_eq!(u.operator, UpdateOperator::Decrement);
                assert!(!u.prefix);
            }
            other => panic!("expected UpdateExpression, got {other:?}"),
        }
    }

    /// `++a` bridges to a prefix `UpdateExpression` (Increment) — NOT a
    /// `UnaryExpression` and NOT a silently-dropped operand.
    #[test]
    fn prefix_increment() {
        match sole_expr("++a;") {
            Expression::UpdateExpression(u) => {
                assert_eq!(u.operator, UpdateOperator::Increment);
                assert!(u.prefix, "++a is prefix");
                assert!(matches!(&*u.argument, Expression::Identifier(i) if i.name == "a"));
            }
            other => panic!("expected UpdateExpression, got {other:?}"),
        }
    }

    /// `--a` bridges to a prefix `UpdateExpression` (Decrement).
    #[test]
    fn prefix_decrement() {
        match sole_expr("--a;") {
            Expression::UpdateExpression(u) => {
                assert_eq!(u.operator, UpdateOperator::Decrement);
                assert!(u.prefix);
            }
            other => panic!("expected UpdateExpression, got {other:?}"),
        }
    }

    /// A member operand round-trips: `a.b++` is a postfix update over the
    /// member access (a valid writable reference).
    #[test]
    fn postfix_increment_on_member() {
        match sole_expr("a.b++;") {
            Expression::UpdateExpression(u) => {
                assert!(!u.prefix);
                assert_eq!(u.operator, UpdateOperator::Increment);
                assert!(matches!(&*u.argument, Expression::MemberExpression(_)));
            }
            other => panic!("expected UpdateExpression, got {other:?}"),
        }
    }

    /// A bare `postfix_expression` with no `++`/`--` still passes through to
    /// its operand (no regression to the pass-through path).
    #[test]
    fn bare_operand_still_passes_through() {
        assert!(matches!(sole_expr("a;"), Expression::Identifier(i) if i.name == "a"));
    }

    // ---- NewExpression (CLOC12.159 PR2, closes gap-160) ----------------

    /// `new X()` bridges to a `NewExpression` with the identifier callee and an
    /// empty argument list — no longer declined to `UnsupportedSyntax`.
    #[test]
    fn new_identifier_no_args() {
        match sole_expr("new X();") {
            Expression::NewExpression(n) => {
                assert!(matches!(&*n.callee, Expression::Identifier(i) if i.name == "X"));
                assert!(n.arguments.is_empty());
            }
            other => panic!("expected NewExpression, got {other:?}"),
        }
    }

    /// `new X(1, 2)` carries both arguments through in order.
    #[test]
    fn new_with_args() {
        match sole_expr("new X(1, 2);") {
            Expression::NewExpression(n) => {
                assert!(matches!(&*n.callee, Expression::Identifier(i) if i.name == "X"));
                assert_eq!(n.arguments.len(), 2);
                assert!(matches!(&n.arguments[0], Expression::NumericLiteral(l) if l.value == 1.0));
                assert!(matches!(&n.arguments[1], Expression::NumericLiteral(l) if l.value == 2.0));
            }
            other => panic!("expected NewExpression, got {other:?}"),
        }
    }

    /// A member-chain callee is preserved: `new a.b(c)` constructs via `a.b`.
    #[test]
    fn new_member_callee() {
        match sole_expr("new a.b(c);") {
            Expression::NewExpression(n) => {
                assert!(matches!(&*n.callee, Expression::MemberExpression(_)));
                assert_eq!(n.arguments.len(), 1);
            }
            other => panic!("expected NewExpression, got {other:?}"),
        }
    }

    /// A bare `new X` (no parens) is the same program as `new X()` — it bridges
    /// to a `NewExpression` with an EMPTY argument list (never dropped).
    #[test]
    fn bare_new_no_parens() {
        match sole_expr("new X;") {
            Expression::NewExpression(n) => {
                assert!(matches!(&*n.callee, Expression::Identifier(i) if i.name == "X"));
                assert!(n.arguments.is_empty());
            }
            other => panic!("expected NewExpression, got {other:?}"),
        }
    }

    /// `new X().y` — a member access on the construction result. The argumented
    /// `new` is the member object and the `.y` suffix folds onto it.
    #[test]
    fn new_then_member_access() {
        match sole_expr("new X().y;") {
            Expression::MemberExpression(m) => {
                assert!(matches!(&*m.object, Expression::NewExpression(_)));
                assert!(matches!(&*m.property, Expression::Identifier(i) if i.name == "y"));
                assert!(!m.computed);
            }
            other => panic!("expected MemberExpression over a NewExpression, got {other:?}"),
        }
    }

    /// `new` nests: `new new X()` constructs with the result of an inner
    /// construction — both bridge to `NewExpression` (never declined).
    #[test]
    fn nested_new() {
        match sole_expr("new new X();") {
            Expression::NewExpression(outer) => {
                assert!(matches!(&*outer.callee, Expression::NewExpression(_)));
            }
            other => panic!("expected nested NewExpression, got {other:?}"),
        }
    }

    // ---- SpreadElement (CLOC12.162 PR2, closes gap-163) ----------------

    /// `f(...a)` bridges to a `CallExpression` whose sole argument is a
    /// `SpreadElement` wrapping the identifier `a` (no longer declined).
    #[test]
    fn call_with_spread_arg() {
        match sole_expr("f(...a);") {
            Expression::CallExpression(c) => {
                assert_eq!(c.arguments.len(), 1, "one (spread) argument");
                match &c.arguments[0] {
                    Expression::SpreadElement(s) => {
                        assert!(matches!(&*s.argument, Expression::Identifier(i) if i.name == "a"));
                    }
                    other => panic!("expected SpreadElement argument, got {other:?}"),
                }
            }
            other => panic!("expected CallExpression, got {other:?}"),
        }
    }

    /// `f(a, ...b, c)` preserves arity and position — a plain arg, a spread, a
    /// plain arg, in order.
    #[test]
    fn call_spread_interleaved_preserves_arity() {
        match sole_expr("f(a, ...b, c);") {
            Expression::CallExpression(c) => {
                assert_eq!(c.arguments.len(), 3, "three arguments in order");
                assert!(matches!(&c.arguments[0], Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(&c.arguments[1], Expression::SpreadElement(s)
                    if matches!(&*s.argument, Expression::Identifier(i) if i.name == "b")));
                assert!(matches!(&c.arguments[2], Expression::Identifier(i) if i.name == "c"));
            }
            other => panic!("expected CallExpression, got {other:?}"),
        }
    }

    /// `new X(...a)` bridges to a `NewExpression` whose sole argument is a
    /// `SpreadElement` (the `new` argument list reuses `convert_arguments`).
    #[test]
    fn new_with_spread_arg() {
        match sole_expr("new X(...a);") {
            Expression::NewExpression(n) => {
                assert_eq!(n.arguments.len(), 1, "one (spread) argument");
                assert!(matches!(&n.arguments[0], Expression::SpreadElement(s)
                    if matches!(&*s.argument, Expression::Identifier(i) if i.name == "a")));
            }
            other => panic!("expected NewExpression, got {other:?}"),
        }
    }

    /// `[...a]` bridges to an `ArrayExpression` whose sole element is a
    /// `SpreadElement` wrapping `a`.
    #[test]
    fn array_with_spread_element() {
        match sole_expr("[...a];") {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 1, "one (spread) element");
                match &a.elements[0] {
                    Some(Expression::SpreadElement(s)) => {
                        assert!(matches!(&*s.argument, Expression::Identifier(i) if i.name == "a"));
                    }
                    other => panic!("expected Some(SpreadElement), got {other:?}"),
                }
            }
            other => panic!("expected ArrayExpression, got {other:?}"),
        }
    }

    /// `[1, ...a, 2]` keeps element count and order: literal, spread, literal.
    #[test]
    fn array_spread_interleaved_preserves_count() {
        match sole_expr("[1, ...a, 2];") {
            Expression::ArrayExpression(a) => {
                assert_eq!(a.elements.len(), 3, "three elements in order");
                assert!(matches!(&a.elements[0], Some(Expression::NumericLiteral(n)) if n.value == 1.0));
                assert!(matches!(&a.elements[1], Some(Expression::SpreadElement(s))
                    if matches!(&*s.argument, Expression::Identifier(i) if i.name == "a")));
                assert!(matches!(&a.elements[2], Some(Expression::NumericLiteral(n)) if n.value == 2.0));
            }
            other => panic!("expected ArrayExpression, got {other:?}"),
        }
    }

    /// Guard: a NON-spread argument still bridges to a bare expression, not a
    /// `SpreadElement` (the `has_token("...")` gate is spread-specific).
    #[test]
    fn plain_call_arg_is_not_spread() {
        match sole_expr("f(a);") {
            Expression::CallExpression(c) => {
                assert_eq!(c.arguments.len(), 1);
                assert!(matches!(&c.arguments[0], Expression::Identifier(i) if i.name == "a"));
            }
            other => panic!("expected CallExpression, got {other:?}"),
        }
    }

    // ---- SequenceExpression (CLOC12.160 PR2, closes gap-161) -----------

    /// `a, b, c` bridges to a `SequenceExpression` holding the three operands
    /// in source order — no longer declined to `UnsupportedSyntax`.
    #[test]
    fn sequence_three_operands() {
        match sole_expr("a, b, c;") {
            Expression::SequenceExpression(s) => {
                assert_eq!(s.expressions.len(), 3);
                assert!(matches!(&s.expressions[0], Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(&s.expressions[1], Expression::Identifier(i) if i.name == "b"));
                assert!(matches!(&s.expressions[2], Expression::Identifier(i) if i.name == "c"));
            }
            other => panic!("expected SequenceExpression, got {other:?}"),
        }
    }

    /// A two-operand sequence with a foldable operand keeps both operands (the
    /// bridge does not fold — that is a pass's job): `a, 1 + 2`.
    #[test]
    fn sequence_two_operands_preserved() {
        match sole_expr("a, 1 + 2;") {
            Expression::SequenceExpression(s) => {
                assert_eq!(s.expressions.len(), 2);
                assert!(matches!(&s.expressions[0], Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(&s.expressions[1], Expression::BinaryExpression(_)));
            }
            other => panic!("expected SequenceExpression, got {other:?}"),
        }
    }

    /// A parenthesised sequence `(a, b)` inside an expression bridges to a
    /// `SequenceExpression` — here as the RHS of an assignment `x = (a, b)`.
    #[test]
    fn parenthesised_sequence_as_assignment_rhs() {
        match sole_expr("x = (a, b);") {
            Expression::AssignmentExpression(a) => {
                assert!(matches!(&*a.right, Expression::SequenceExpression(s) if s.expressions.len() == 2));
            }
            other => panic!("expected AssignmentExpression with a sequence RHS, got {other:?}"),
        }
    }

    /// A single expression is NOT wrapped in a sequence — the one-operand path
    /// still passes the operand through unchanged.
    #[test]
    fn single_operand_not_a_sequence() {
        assert!(matches!(sole_expr("a;"), Expression::Identifier(i) if i.name == "a"));
    }

    // ---- Generators + yield (CLOC12.163 PR2, closes gap-164) -----------

    use coding_adventures_javascript_ast::statement::TaggedStatement;

    /// Bridge `src`, expect a single generator/function *declaration*, and
    /// return `(generator_flag, first_body_expression)`.
    fn gen_decl(src: &str) -> (bool, Expression) {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) => {
                let expr = match &f.body.body[0] {
                    Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                        es.expression.clone()
                    }
                    other => panic!("expected an expression statement in body, got {other:?}"),
                };
                (f.generator, expr)
            }
            other => panic!("expected a FunctionDeclaration, got {other:?}"),
        }
    }

    /// `function*g(){yield x;}` bridges to a *generator* `FunctionDeclaration`
    /// (no longer declined) whose body holds a non-delegating `YieldExpression`
    /// over `x`.
    #[test]
    fn generator_declaration_with_yield() {
        let (is_gen, expr) = gen_decl("function*g(){yield x;}");
        assert!(is_gen, "function* must set the generator flag");
        match expr {
            Expression::YieldExpression(y) => {
                assert!(!y.delegate, "`yield x` is not delegating");
                assert!(matches!(y.argument.as_deref(),
                    Some(Expression::Identifier(i)) if i.name == "x"));
            }
            other => panic!("expected YieldExpression, got {other:?}"),
        }
    }

    /// `function*g(){yield* xs;}` bridges to a **delegating** `YieldExpression`
    /// (the `*` sets `delegate`).
    #[test]
    fn generator_declaration_with_delegate_yield() {
        let (is_gen, expr) = gen_decl("function*g(){yield* xs;}");
        assert!(is_gen);
        match expr {
            Expression::YieldExpression(y) => {
                assert!(y.delegate, "`yield*` is delegating");
                assert!(matches!(y.argument.as_deref(),
                    Some(Expression::Identifier(i)) if i.name == "xs"));
            }
            other => panic!("expected delegating YieldExpression, got {other:?}"),
        }
    }

    /// The yield operand is bridged in full — `yield a + b` carries a
    /// `BinaryExpression` operand (the bridge does not fold; that is a pass's
    /// job), so a downstream fold pass can still optimise it.
    #[test]
    fn yield_binary_operand_bridged() {
        let (_, expr) = gen_decl("function*g(){yield a + b;}");
        match expr {
            Expression::YieldExpression(y) => {
                assert!(matches!(y.argument.as_deref(), Some(Expression::BinaryExpression(_))));
            }
            other => panic!("expected YieldExpression, got {other:?}"),
        }
    }

    /// A **generator expression** in value position (`x = function*(){yield 1;}`)
    /// bridges to a `FunctionExpression` with `generator == true` — no longer
    /// declined.
    #[test]
    fn generator_expression_in_value_position() {
        match sole_expr("x = function*(){yield 1;};") {
            Expression::AssignmentExpression(a) => match &*a.right {
                Expression::FunctionExpression(f) => {
                    assert!(f.generator, "function* expression must set the generator flag");
                }
                other => panic!("expected a generator FunctionExpression RHS, got {other:?}"),
            },
            other => panic!("expected AssignmentExpression, got {other:?}"),
        }
    }

    /// A **plain** (non-generator) function declaration keeps `generator ==
    /// false` — the `*`-detection does not misfire on `function g(){}`.
    #[test]
    fn plain_function_is_not_a_generator() {
        let p = bridge_ok("function g(){return 1;}");
        match &p.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) => {
                assert!(!f.generator, "a plain function is not a generator");
            }
            other => panic!("expected a FunctionDeclaration, got {other:?}"),
        }
    }
}
