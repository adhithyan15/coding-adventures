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
        AssignmentPattern, BindingTarget, ClassDeclaration, Declaration, ExportAllDeclaration,
        ExportDefaultDeclaration, ExportDefaultKind, ExportNamedDeclaration, ExportSpecifier,
        FunctionDeclaration, FunctionParam, ImportDeclaration, ImportSpecifier, RestElement,
        VarKind, VariableDeclaration, VariableDeclarator,
    },
    expression::{
        ArrayExpression, ArrowBody, ArrowFunctionExpression, AssignmentExpression,
        AssignmentOperator, AssignmentTarget,
        BigIntLiteral, BinaryExpression, BinaryOperator, BooleanLiteral, CallExpression,
        ClassExpression, ClassMember, MethodDefinition, MethodKind, PropertyDefinition,
        ConditionalExpression, Expression, FunctionExpression, Identifier, LogicalExpression,
        LogicalOperator,
        ChainExpression, ImportExpression, ImportMeta, MemberExpression, NewExpression, NewTarget, NullLiteral, NumericLiteral, ObjectExpression, ObjectMember,
        OptionalCallExpression, OptionalMemberExpression, PrivateName, Property,
        PropertyKey, PropertyKind, RegExpLiteral, SequenceExpression, SpreadElement, StringLiteral, TaggedTemplateExpression,
        TemplateElement, TemplateLiteral,
        UnaryExpression, UnaryOperator,
        Super, ThisExpression, UndefinedLiteral, UpdateExpression, UpdateOperator, YieldExpression,
    },
    statement::{
        BlockStatement, BreakStatement, CatchClause, ContinueStatement, DebuggerStatement,
        DoWhileStatement, EmptyStatement, ExpressionStatement, ForInStatement, ForInit,
        ForOfStatement, ForStatement, IfStatement, LabeledStatement, ReturnStatement, Statement,
        SwitchCase, SwitchStatement, ThrowStatement, TryStatement, WhileStatement, WithStatement,
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

/// The leftmost terminal token value in a subtree, descending through Node
/// children to the first `Token` leaf (depth-first, left to right). Returns
/// `None` for a subtree with no tokens at all. Used to tell a bare block body
/// `=> {…}` (leftmost `{`) from a parenthesised object body `=> ({…})`
/// (leftmost `(`) — see [`convert_arrow_function`].
fn leftmost_token(node: &GrammarASTNode) -> Option<&str> {
    for c in &node.children {
        match c {
            ASTNodeOrToken::Token(t) => return Some(t.value.as_str()),
            ASTNodeOrToken::Node(n) => {
                if let Some(v) = leftmost_token(n) {
                    return Some(v);
                }
            }
        }
    }
    None
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
        // A class *declaration* (`class C { … }`) — CLOC12.174 PR2. The grammar
        // wraps it in `decorated_class_declaration` (the outer rule that would
        // also carry `@decorator`s); the bare `class_declaration` alternative is
        // handled too for robustness. A decorated form with actual decorators
        // carries extra child nodes this slice does not model, so it DECLINES
        // (safe WHITESPACE_ONLY fallback).
        "decorated_class_declaration" => match node_children(child).as_slice() {
            [cd] if cd.rule_name == "class_declaration" => {
                let decl = convert_class_declaration(cd)?;
                Ok(ProgramItem::Declaration(Declaration::ClassDeclaration(decl)))
            }
            _ => Err(unsupported(child)),
        },
        "class_declaration" => {
            let decl = convert_class_declaration(child)?;
            Ok(ProgramItem::Declaration(Declaration::ClassDeclaration(decl)))
        }
        // An ES-module `import` declaration (CLOC12.188 PR2). Recognised shapes:
        // side-effect (`import "y"`), default (`import x from "y"`), namespace
        // (`import * as ns from "y"`), and named (`import {a, b as c} from "y"`),
        // plus default-plus-named (`import x, {a} from "y"`). Anything the
        // converter does not recognise DECLINES to WHITESPACE_ONLY.
        "import_declaration" => {
            let decl = convert_import_declaration(child)?;
            Ok(ProgramItem::Declaration(Declaration::ImportDeclaration(decl)))
        }
        // An ES-module `export` declaration (CLOC12.189 PR2). Recognised shapes:
        // named (`export {a, b as c}`), re-export (`export {a} from "y"`),
        // export-all (`export * from "y"`), default (`export default …`), and
        // declaration exports (`export const/var/function/class …`). Anything
        // unrecognised — e.g. `export * as ns from "y"` (grammar gap) — DECLINES.
        "export_declaration" => Ok(ProgramItem::Declaration(convert_export_declaration(child)?)),
        "statement" => {
            let stmt = convert_statement(child)?;
            Ok(ProgramItem::Statement(stmt))
        }
        // variable_statement / lexical_declaration land inside statement
        _ => Err(unsupported(child)),
    }
}

/// Convert an `import_declaration` grammar node into an [`ImportDeclaration`]
/// (CLOC12.188 PR2). Grammar shape (verified by a parse-tree probe):
///
/// ```text
///   import_declaration = Token("import"),
///                        ( module_specifier                    // side-effect
///                        | import_clause , from_clause ),      // with bindings
///                        Token(";") ;
///   import_clause  = [ default_import ] , [ Token(",") ] ,
///                    [ namespace_import | named_imports ] ;
///   default_import   = Token(name) ;                            // `x`
///   namespace_import = Token("*"), Token("as"), Token(name) ;   // `* as ns`
///   named_imports    = Token("{"),
///                      { import_specifier , [ Token(",") ] },
///                      Token("}") ;
///   import_specifier = Token(name) , [ Token("as"), Token(name) ] ; // `a`/`a as c`
/// ```
///
/// The source string rides a `String` token inside `module_specifier` (the
/// side-effect form) or `from_clause` (the with-bindings form); the lexer
/// stores it *unquoted*, so we rebuild the raw `"…"` form for the
/// [`StringLiteral`]. `import x, * as ns from "y"` is a grammar gap (the parser
/// rejects a default+namespace combination) and never reaches here. Any shape
/// the arms below do not recognise DECLINES via `unsupported` (a safe
/// WHITESPACE_ONLY fallback), never a mis-bridge.
fn convert_import_declaration(node: &GrammarASTNode) -> Result<ImportDeclaration, BridgeError> {
    let kids = node_children(node);
    let mut specifiers: Vec<ImportSpecifier> = Vec::new();
    let source_node = match kids.as_slice() {
        // Side-effect import `import "y";` — no clause; the source is the direct
        // `module_specifier` child.
        [ms] if ms.rule_name == "module_specifier" => ms,
        // Import with bindings `import <clause> from "y";`.
        [clause, from]
            if clause.rule_name == "import_clause" && from.rule_name == "from_clause" =>
        {
            for part in node_children(clause) {
                match part.rule_name.as_str() {
                    // `x` — default binding.
                    "default_import" => match token_vals(part).as_slice() {
                        [name] => specifiers.push(ImportSpecifier::Default(Identifier {
                            cv: None,
                            name: (*name).to_string(),
                        })),
                        _ => return Err(unsupported(part)),
                    },
                    // `* as ns` — namespace binding; the local name is the last token.
                    "namespace_import" => match token_vals(part).as_slice() {
                        [star, as_kw, ns] if *star == "*" && *as_kw == "as" => {
                            specifiers.push(ImportSpecifier::Namespace(Identifier {
                                cv: None,
                                name: (*ns).to_string(),
                            }))
                        }
                        _ => return Err(unsupported(part)),
                    },
                    // `{a, b as c}` — zero or more named specifiers.
                    "named_imports" => {
                        for spec in node_children(part) {
                            if spec.rule_name != "import_specifier" {
                                return Err(unsupported(spec));
                            }
                            // `a` → imported == local == a ;
                            // `a as c` → imported = a, local = c.
                            let (imported, local) = match token_vals(spec).as_slice() {
                                [a] => ((*a).to_string(), (*a).to_string()),
                                [a, as_kw, c] if *as_kw == "as" => {
                                    ((*a).to_string(), (*c).to_string())
                                }
                                _ => return Err(unsupported(spec)),
                            };
                            specifiers.push(ImportSpecifier::Named {
                                imported: Identifier { cv: None, name: imported },
                                local: Identifier { cv: None, name: local },
                            });
                        }
                    }
                    _ => return Err(unsupported(part)),
                }
            }
            from
        }
        _ => return Err(unsupported(node)),
    };
    let source = import_source(source_node).ok_or_else(|| unsupported(source_node))?;
    Ok(ImportDeclaration { cv: None, specifiers, source })
}

/// Pull the module-specifier string out of a `module_specifier` or `from_clause`
/// node: the first `String`-typed token among the node's direct children. The
/// lexer stores the value unquoted (`y`, not `"y"`), so we rebuild a double-
/// quoted `raw` for the [`StringLiteral`]; the emitter re-derives the quotes
/// from `value`, so `raw` is only kept for round-trip fidelity.
fn import_source(node: &GrammarASTNode) -> Option<StringLiteral> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.type_ == TokenType::String => Some(StringLiteral {
            cv: t.cv.clone(),
            value: t.value.clone(),
            raw: format!("\"{}\"", t.value),
        }),
        _ => None,
    })
}

/// Convert an `export_declaration` grammar node into a `Declaration` (one of the
/// three `Export*` variants) — CLOC12.189 PR2. Grammar shape (verified by a
/// parse-tree probe):
///
/// ```text
///   export_declaration =
///       Token("export"),
///       ( named_exports [ , from_clause ]                    // export {a}[from"y"]
///       | Token("*"), from_clause                            // export * from "y"
///       | Token("default"), ( assignment_expression          // export default 1
///                           | function_declaration            // export default fn
///                           | decorated_class_declaration )   // export default class
///       | lexical_declaration | variable_statement            // export const/var …
///       | function_declaration                                // export function …
///       | decorated_class_declaration ),                      // export class …
///       Token(";") ? ;
///   named_exports    = Token("{"), { export_specifier, [Token(",")] }, Token("}");
///   export_specifier = Token(name) , [ Token("as"), Token(name) ]; // `a`/`a as c`
/// ```
///
/// The inner declaration/expression is bridged by reusing the existing
/// `convert_*` helpers, so every construct those already model works inside an
/// `export`. `export * as ns from "y"` is a grammar gap (rejected at parse) and
/// any unrecognised shape DECLINES via `unsupported` (safe WHITESPACE_ONLY
/// fallback), never a mis-bridge.
fn convert_export_declaration(node: &GrammarASTNode) -> Result<Declaration, BridgeError> {
    let kids = node_children(node);

    // `export default <expr | function | class>`.
    if has_token(node, "default") {
        let child = kids
            .first()
            .ok_or_else(|| internal(node, "export default: missing operand"))?;
        let kind = match child.rule_name.as_str() {
            "function_declaration" | "generator_declaration" => {
                ExportDefaultKind::FunctionDeclaration(convert_function_declaration(child)?)
            }
            "decorated_class_declaration" => match node_children(child).as_slice() {
                [cd] if cd.rule_name == "class_declaration" => {
                    ExportDefaultKind::ClassDeclaration(convert_class_declaration(cd)?)
                }
                _ => return Err(unsupported(child)),
            },
            "class_declaration" => {
                ExportDefaultKind::ClassDeclaration(convert_class_declaration(child)?)
            }
            // Anything else is an expression operand (`export default 1`,
            // `export default foo()`); `convert_expression` dispatches on the
            // expression rule (`assignment_expression`, …).
            _ => ExportDefaultKind::Expression(Box::new(convert_expression(child)?)),
        };
        return Ok(Declaration::ExportDefaultDeclaration(ExportDefaultDeclaration {
            cv: None,
            declaration: kind,
        }));
    }

    // `export * from "y"` — the `*` is a token; the sole node child is the
    // `from_clause`. (`export * as ns from "y"` fails at parse, so a namespace
    // binding never reaches here → `exported` is always None.)
    if has_token(node, "*") {
        let from = kids
            .iter()
            .find(|n| n.rule_name == "from_clause")
            .ok_or_else(|| unsupported(node))?;
        let source = import_source(from).ok_or_else(|| unsupported(from))?;
        return Ok(Declaration::ExportAllDeclaration(ExportAllDeclaration {
            cv: None,
            exported: None,
            source,
        }));
    }

    // `export { a, b as c }` / `export { a } from "y"` — a named-specifier
    // export, optionally re-exporting from another module.
    if let Some(named) = kids.iter().find(|n| n.rule_name == "named_exports") {
        let mut specifiers: Vec<ExportSpecifier> = Vec::new();
        for spec in node_children(named) {
            if spec.rule_name != "export_specifier" {
                return Err(unsupported(spec));
            }
            // `a` → local == exported == a ; `a as c` → local = a, exported = c.
            let (local, exported) = match token_vals(spec).as_slice() {
                [a] => ((*a).to_string(), (*a).to_string()),
                [a, as_kw, c] if *as_kw == "as" => ((*a).to_string(), (*c).to_string()),
                _ => return Err(unsupported(spec)),
            };
            specifiers.push(ExportSpecifier {
                local: Identifier { cv: None, name: local },
                exported: Identifier { cv: None, name: exported },
            });
        }
        let source = kids
            .iter()
            .find(|n| n.rule_name == "from_clause")
            .and_then(|f| import_source(f));
        return Ok(Declaration::ExportNamedDeclaration(ExportNamedDeclaration {
            cv: None,
            declaration: None,
            specifiers,
            source,
        }));
    }

    // `export const/let/var …` / `export function …` / `export class …` — a
    // declaration export: the sole node child is the inner declaration, bridged
    // by the existing converter for its kind.
    let inner = kids
        .first()
        .ok_or_else(|| internal(node, "export declaration: missing inner declaration"))?;
    let decl = match inner.rule_name.as_str() {
        "lexical_declaration" => {
            Declaration::VariableDeclaration(convert_lexical_declaration(inner)?)
        }
        "variable_statement" => {
            Declaration::VariableDeclaration(convert_variable_statement(inner)?)
        }
        "function_declaration" | "generator_declaration" => {
            Declaration::FunctionDeclaration(convert_function_declaration(inner)?)
        }
        "decorated_class_declaration" => match node_children(inner).as_slice() {
            [cd] if cd.rule_name == "class_declaration" => {
                Declaration::ClassDeclaration(convert_class_declaration(cd)?)
            }
            _ => return Err(unsupported(inner)),
        },
        "class_declaration" => Declaration::ClassDeclaration(convert_class_declaration(inner)?),
        _ => return Err(unsupported(inner)),
    };
    Ok(Declaration::ExportNamedDeclaration(ExportNamedDeclaration {
        cv: None,
        declaration: Some(Box::new(decl)),
        specifiers: Vec::new(),
        source: None,
    }))
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
        // with_statement = "with" LPAREN expression RPAREN statement (CLOC12.187).
        // The atomic node + emitter + pass traversal landed in PR1, and the
        // renaming-soundness gate (rename passes decline when a `with` is
        // present) landed in PR2a — so bridging it here is sound.
        "with_statement" => convert_with_statement(child).map(Statement::with_statement),
        // Phase 2+ — not yet in the typed AST
        "for_await_of_statement" | "using_declaration" | "await_using_declaration" => {
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

fn convert_with_statement(node: &GrammarASTNode) -> Result<WithStatement, BridgeError> {
    // with_statement = "with" LPAREN expression RPAREN statement (CLOC12.187).
    // Node children: [expression, statement] — the injected object first, the
    // body second. Structurally identical to `while_statement`; the difference
    // is purely semantic (`with` splices the object onto the scope chain), and
    // that semantics is handled downstream by the renaming-soundness gate
    // (`program_contains_with_statement`) rather than here.
    let nodes = node_children(node);
    if nodes.len() < 2 {
        return Err(internal(node, "with_statement needs 2 node children"));
    }
    Ok(WithStatement {
        cv: None,
        object: convert_expression(nodes[0])?,
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

    // Phase 0: init — a `var` declaration list, a `let`/`const` binding list, or
    // a bare expression.
    if let Some(&n) = phase_nodes[0].first() {
        match n.rule_name.as_str() {
            "variable_declaration_list" => {
                let decl = convert_var_decl_list(n, VarKind::Var)?;
                init = Some(ForInit::VariableDeclaration(decl));
            }
            // `for (let/const i = 0; …)` (CLOC12.186). The grammar inlines the
            // lexical declaration into the for-header: the `let`/`const` keyword
            // is a direct Token child of the `for_statement` (so `has_token`
            // finds it), and the bindings are a bare `binding_list` node whose
            // children are `lexical_binding` nodes — the same shape
            // `convert_lexical_declaration` reads, so we reuse
            // `convert_variable_declarator` on them.
            "binding_list" => {
                let kind = if has_token(node, "const") {
                    VarKind::Const
                } else {
                    VarKind::Let
                };
                let declarations: Result<Vec<VariableDeclarator>, _> =
                    node_children(n).into_iter().map(convert_variable_declarator).collect();
                init = Some(ForInit::VariableDeclaration(VariableDeclaration {
                    cv: None,
                    kind,
                    declarations: declarations?,
                }));
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
        node_children(node).into_iter().map(convert_statement).collect();
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
        .map(convert_variable_declarator)
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
        .map(convert_variable_declarator)
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
// ClassExpression (CLOC12.173 PR2 — bridge enable, gap-167)
// ---------------------------------------------------------------------

/// Convert a `class_expression` grammar node into a [`ClassExpression`].
///
/// The parse-tree shape (confirmed by dumping the grammar parser's output —
/// see the throwaway probe removed with this PR) is a *flat* child list:
///
/// ```text
///   class_expression = [ Token("class"),
///                        Token(NAME)?,        // the class name, if any
///                        Node(class_heritage)?,
///                        Node(class_body) ]
/// ```
///
/// - **Name.** The single direct-child `Token` other than the `class` keyword
///   is the class name. It is `None` for an anonymous `class {}` /
///   `class extends B {}` and `Some` for a named `class C {}`. (Every other
///   token lives *inside* the `class_heritage` / `class_body` child nodes, so a
///   scan of the class node's own token children never confuses them.)
/// - **Heritage.** `class_heritage = [ Token("extends"), <operand> ]`. The
///   operand is either a bare `Token(NAME)` (for `extends B`) or a
///   `Node(left_hand_side_expression)` (for `extends ns.B`). We convert a
///   child node with [`convert_expression`]; a lone NAME token becomes an
///   [`Identifier`]. Anything else (e.g. the grammar flattens `extends mix(B)`
///   into two ambiguous NAME tokens) DECLINES via `UnsupportedSyntax` rather
///   than risk mis-reading the super-class — the file then falls back to
///   WHITESPACE_ONLY, never a miscompile.
/// - **Body.** `class_body`'s `class_element` children each become one
///   [`ClassMember`] via [`convert_class_element`].
fn convert_class_expression(node: &GrammarASTNode) -> Result<ClassExpression, BridgeError> {
    // Name: the only direct-child token that is not the `class` keyword.
    let id = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value != "class" => Some(t.value.clone()),
            _ => None,
        })
        .map(|name| Identifier { cv: None, name });

    // Heritage (`extends <operand>`), if present.
    let mut super_class: Option<Box<Expression>> = None;
    if let Some(heritage) = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "class_heritage" => Some(n),
            _ => None,
        })
    {
        super_class = Some(Box::new(convert_class_heritage(heritage)?));
    }

    // Body: iterate `class_element` children of the `class_body` node.
    let body_node = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "class_body" => Some(n),
            _ => None,
        })
        .ok_or_else(|| internal(node, "class_expression: missing class_body"))?;

    let mut body = Vec::new();
    for el in node_children(body_node) {
        if el.rule_name == "class_element" {
            body.push(convert_class_element(el)?);
        }
    }

    Ok(ClassExpression { cv: None, id, super_class, body })
}

/// Convert a `class_declaration` grammar node into a [`ClassDeclaration`] — the
/// **statement** form (`class C { … }`), CLOC12.174 PR2.
///
/// The parse-tree shape (confirmed by dumping the grammar parser's output — see
/// the throwaway probe removed with this PR) is the *same flat child list* as
/// `class_expression`, with **one difference: the name is required**:
///
/// ```text
///   class_declaration = [ Token("class"),
///                         Token(NAME),          // REQUIRED — a declaration binds a name
///                         Node(class_heritage)?,
///                         Node(class_body) ]
/// ```
///
/// (At the `source_element` level this node is wrapped in
/// `decorated_class_declaration`, unwrapped by [`convert_source_element`].)
/// Heritage and body are byte-identical to the expression form, so this reuses
/// [`convert_class_heritage`] and [`convert_class_element`] unchanged — only the
/// name handling differs: a missing name is not a valid declaration, so it
/// DECLINES (safe WHITESPACE_ONLY fallback) rather than fabricating an empty id.
fn convert_class_declaration(node: &GrammarASTNode) -> Result<ClassDeclaration, BridgeError> {
    // Name: the required class name — the single direct-child token that is not
    // the `class` keyword. Unlike `class_expression` (optional id), a *missing*
    // name here DECLINES: a class declaration with no name is not valid syntax
    // this slice represents.
    let id = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value != "class" => Some(t.value.clone()),
            _ => None,
        })
        .map(|name| Identifier { cv: None, name })
        .ok_or_else(|| unsupported(node))?;

    // Heritage (`extends <operand>`), if present — same converter as the
    // expression form.
    let mut super_class: Option<Box<Expression>> = None;
    if let Some(heritage) = node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "class_heritage" => Some(n),
        _ => None,
    }) {
        super_class = Some(Box::new(convert_class_heritage(heritage)?));
    }

    // Body: iterate `class_element` children of the `class_body` node — same
    // converter as the expression form.
    let body_node = node
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "class_body" => Some(n),
            _ => None,
        })
        .ok_or_else(|| internal(node, "class_declaration: missing class_body"))?;

    let mut body = Vec::new();
    for el in node_children(body_node) {
        if el.rule_name == "class_element" {
            body.push(convert_class_element(el)?);
        }
    }

    Ok(ClassDeclaration { cv: None, id, super_class, body })
}

/// Convert a `class_heritage` (`extends <operand>`) node into the super-class
/// [`Expression`]. See the shape note on [`convert_class_expression`].
fn convert_class_heritage(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // Prefer a child *node* operand (`extends ns.B` → left_hand_side_expression).
    if let Some(operand) = node_children(node).into_iter().next() {
        return convert_expression(operand);
    }
    // Otherwise a lone NAME token (`extends B`). Exactly one non-`extends`
    // token is a clean identifier; anything else (the grammar's flattened
    // `extends mix(B)` yields several tokens) is ambiguous — DECLINE.
    let names: Vec<&String> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value != "extends" => Some(&t.value),
            _ => None,
        })
        .collect();
    match names.as_slice() {
        [name] => Ok(Expression::Identifier(Identifier { cv: None, name: (*name).clone() })),
        _ => Err(unsupported(node)),
    }
}

/// Convert one `class_element` into a [`ClassMember`].
///
/// ```text
///   class_element = [ Token("static")? , Node(method_definition) , Token(";") ]
///                 | Node(class_field_declaration)          // CLOC12.175 PR2
///                 | …declined shapes (async_method, private, static_block, ;)
/// ```
///
/// A *method* member is `[Token("static")?, method_definition, ";"]` — a leading
/// bare `Token("static")` marks it static; the `method_definition` child carries
/// the rest. A *field* member is a single `class_field_declaration` node that
/// carries its **own** `static` token internally (the grammar does NOT hoist a
/// field's `static` to the `class_element` level, unlike a method's), so the
/// `is_static` scanned here stays `false` for a field and
/// [`convert_class_field`] reads the field's own modifier.
///
/// **`async` lives here, not on `method_definition`.** The grammar attaches an
/// `async` method's `async` keyword to the *`class_element`* (`async m(){}` →
/// `[Token("async"), method_definition]`), unlike the generator `*`, which sits
/// inside `method_definition`. An `async` member carries semantics this slice
/// does not model, so any `class_element` token other than `static` DECLINES —
/// otherwise the `async` would be silently dropped, a semantics-changing
/// miscompile. (Declining the member drops the whole file to WHITESPACE_ONLY.)
///
/// `static_block` (CLOC12.176), `private_method_definition` (CLOC12.178), and
/// `class_field_declaration` (CLOC12.175) member nodes are modelled; an
/// `async_method` node is a form this slice does not model — it DECLINES (safe
/// WHITESPACE_ONLY fallback) rather than surface a mis-emit.
fn convert_class_element(node: &GrammarASTNode) -> Result<ClassMember, BridgeError> {
    // A `class_element` carries at most one *word* modifier — `static` (which
    // we model) or `async` (which we do not). It may also carry a benign
    // separator `;` (a stray semicolon between members). Inspect only the
    // identifier/keyword tokens: `static` sets the flag; any *other* word
    // modifier declines (so `async` is never silently dropped). Punctuation
    // such as `;` is ignored.
    let mut is_static = false;
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if matches!(t.type_, TokenType::Name | TokenType::Keyword) {
                if t.value == "static" {
                    is_static = true;
                } else {
                    return Err(unsupported(node));
                }
            }
        }
    }
    // The member itself is a *single* child node. Four shapes are modelled:
    // a plain `method_definition` (→ `ClassMember::Method`), a
    // `class_field_declaration` (→ `ClassMember::Field`, CLOC12.175 PR2), a
    // `static_block` (→ `ClassMember::StaticBlock`, CLOC12.176 PR2), and a
    // `private_method_definition` (→ `ClassMember::Method` with a private key,
    // CLOC12.178 PR1). The grammar's `async_method` node is a form this slice
    // does not represent, so it DECLINES (safe WHITESPACE_ONLY fallback).
    // (Because these are *distinct nodes*, not just leading tokens, the node
    // kind must be checked.)
    //
    // A `static_block` — and a `private_method_definition` — carry their own
    // leading `Token("static")` *inside* the node (not on the `class_element`),
    // so the modifier loop above never sees it: `is_static` from `class_element`
    // stays false and each arm reads `static` from inside the member node.
    let member = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "class_element: empty"))?;
    match member.rule_name.as_str() {
        "method_definition" => Ok(ClassMember::Method(convert_method_definition(member, is_static)?)),
        "class_field_declaration" => Ok(ClassMember::Field(convert_class_field(member)?)),
        "static_block" => Ok(ClassMember::StaticBlock(convert_static_block(member)?)),
        "private_method_definition" => {
            Ok(ClassMember::Method(convert_private_method_definition(member)?))
        }
        _ => Err(unsupported(member)),
    }
}

/// Convert a `static_block` node into a [`BlockStatement`] — a class
/// static-initialization block `static { … }` (CLOC12.176 PR2).
///
/// ```text
///   static_block = [ Token("static"), Token("{"),
///                    Node(statement)*,
///                    Token("}") ]
/// ```
///
/// The block body is exactly a statement list — the *same* shape as a plain
/// `{ … }` [`convert_block_statement`] — so each `statement` node child is
/// lowered by the shared [`convert_statement`], covering the full statement
/// surface (including `let` / `const` / `var`, which map to
/// `Statement::Declaration`). Tokens (`static`, the braces) are ignored by
/// [`node_children`], so the leading `static` needs no special handling. An
/// empty block (`static {}`) yields an empty `body`. Because every statement is
/// routed through the shared converter, any unmodelled body statement makes the
/// converter DECLINE (safe WHITESPACE_ONLY fallback) rather than mis-emit.
fn convert_static_block(node: &GrammarASTNode) -> Result<BlockStatement, BridgeError> {
    let stmts: Result<Vec<Statement>, _> =
        node_children(node).into_iter().map(convert_statement).collect();
    Ok(BlockStatement { cv: None, body: stmts? })
}

/// Convert a `class_field_declaration` node into a [`PropertyDefinition`] — a
/// class field `[static] key [= initializer] ;` (CLOC12.175 PR2).
///
/// ```text
///   class_field_declaration = [ Token("static")? ,
///                               (Node(property_name) | Token(PRIVATE_NAME)) ,
///                               [ Token("="), Node(assignment_expression) ]? ,
///                               Token(";") ]
/// ```
///
/// - **`static`** is a bare leading `Token("static")` *inside* this node (unlike
///   a method's `static`, which sits one level up on the `class_element`).
/// - **key** reuses [`convert_property_key`] — the *same* `property_name` node a
///   method key uses, so identifier / string / numeric keys all work, and a
///   computed `[expr]` key lowers to `PropertyKey::Expression` with
///   `computed: true` (CLOC12.180).
/// - **initializer** is the optional `assignment_expression`; a bare field
///   (`y;`) has none and maps to `value: None`.
///
/// A **private** field (`#x`) is a bare `PRIVATE_NAME` token with no
/// `property_name` node. [`private_name_key`] detects it and lowers it to a
/// [`PropertyKey::PrivateName`] (CLOC12.177 PR2); an ordinary keyed field is
/// unchanged. (A private *method* `#m(){}` is a separate `private_method_definition`
/// grammar node, lowered by [`convert_private_method_definition`], CLOC12.178 PR1.)
///
/// If a class-member node carries a bare **private-name** token (`#x`), lower it
/// to a [`PropertyKey::PrivateName`]; otherwise return `None`.
///
/// A private name lexes as a `Name` token whose `type_name` discriminant is
/// `Some("PRIVATE_NAME")` and whose `value` **includes** the leading `#`
/// (e.g. `"#x"`) — unlike a `property_name` node, it is a direct token child of
/// the `class_field_declaration` / `private_method_definition` node. The stored
/// [`PrivateName::name`] omits the `#` (mirroring [`Identifier`]), so we strip it
/// here; the emitter re-adds it. (CLOC12.177 PR2.)
fn private_name_key(node: &GrammarASTNode) -> Option<PropertyKey> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.type_name.as_deref() == Some("PRIVATE_NAME") {
                let name = t.value.strip_prefix('#').unwrap_or(&t.value).to_string();
                return Some(PropertyKey::PrivateName(PrivateName { cv: None, name }));
            }
        }
    }
    None
}

fn convert_class_field(node: &GrammarASTNode) -> Result<PropertyDefinition, BridgeError> {
    // `static` — a bare NAME token before the `property_name` node.
    let mut is_static = false;
    for c in &node.children {
        match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "property_name" => break,
            ASTNodeOrToken::Token(t) if t.value == "static" => is_static = true,
            _ => {}
        }
    }

    // key — normally the `property_name` node. A **private** field (`#x`) instead
    // carries a bare `PRIVATE_NAME` token as a direct child (no `property_name`
    // node), so we detect that first and lower it to `PropertyKey::PrivateName`
    // (CLOC12.177 PR2). An ordinary field falls through to `property_name`.
    let key = if let Some(pk) = private_name_key(node) {
        pk
    } else {
        let key_node = node_children(node)
            .into_iter()
            .find(|n| n.rule_name == "property_name")
            .ok_or_else(|| unsupported(node))?;
        convert_property_key(key_node)?
    };

    // initializer — the optional `assignment_expression` after `=`. A bare field
    // has none → `value: None`.
    let value = match node_children(node)
        .into_iter()
        .find(|n| n.rule_name == "assignment_expression")
    {
        Some(v) => Some(convert_expression(v)?),
        None => None,
    };

    // A computed key `[expr]` bridges to `PropertyKey::Expression` (CLOC12.180),
    // so the `computed` flag tracks exactly that variant.
    let computed = matches!(&key, PropertyKey::Expression(_));
    Ok(PropertyDefinition {
        cv: None,
        key,
        value,
        computed,
        is_static,
    })
}

/// Convert a `method_definition` node into a [`MethodDefinition`].
///
/// ```text
///   method_definition = [ (Token("get") | Token("set"))?,    // accessor kind
///                         Node(property_name),                // the key
///                         Token("("),
///                         Node(formal_parameters | formal_parameter)*,
///                         Token(")"),
///                         Token("{"), Node(function_body)?, Token("}") ]
/// ```
///
/// **Modifier tokens precede the `property_name` node.** `get` / `set` mark an
/// accessor; a `*` marks a **generator method** (`*gen(){}`), bridged since
/// CLOC12.181 by setting the `value`'s `generator` flag so the emitter re-prints
/// the `*` (`yield` inside the body is already a modelled `YieldExpression`, and
/// a generator's `FunctionExpression` value flows through every pass exactly like
/// a top-level `function*` — CLOC12.163). A key literally named `get`
/// (`get(){}`) parses with the `property_name` node *first* — no leading accessor
/// token — so it is correctly an ordinary [`MethodKind::Method`]. (An `async`
/// method is a *separate* grammar node, `async_method`, declined one level up in
/// [`convert_class_element`], so `async` never reaches here.)
///
/// **`constructor`.** A non-static, non-accessor method whose key is the plain
/// identifier `constructor` is [`MethodKind::Constructor`] — the emitter and the
/// rename-properties pass treat it specially (never renamed).
///
/// **Params.** A single parameter parses as a direct `formal_parameter` child;
/// two or more parse under a `formal_parameters` wrapper (mirroring the two
/// grammar shapes). Both are collected. Rest/default/destructured params are
/// declined by the shared [`convert_formal_parameter`] (Phase-1 simple names).
fn convert_method_definition(
    node: &GrammarASTNode,
    is_static: bool,
) -> Result<MethodDefinition, BridgeError> {
    // Collect the modifier tokens that appear *before* the property_name node.
    let mut saw_get = false;
    let mut saw_set = false;
    let mut saw_star = false;
    for c in &node.children {
        match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "property_name" => break,
            ASTNodeOrToken::Token(t) => match t.value.as_str() {
                "get" => saw_get = true,
                "set" => saw_set = true,
                "*" => saw_star = true,
                _ => {}
            },
            _ => {}
        }
    }

    // A generator method (`*gen(){}`) is bridged (CLOC12.181): `saw_star` sets
    // the value's `generator` flag below and the emitter re-prints the `*`. No
    // decline is needed — `yield` inside the body is a modelled `YieldExpression`
    // and a generator `FunctionExpression` flows through every pass exactly like
    // a top-level `function*`.

    let key_node = node_children(node)
        .into_iter()
        .find(|n| n.rule_name == "property_name")
        .ok_or_else(|| internal(node, "method_definition: missing property_name"))?;
    // `convert_property_key` lowers a computed `[expr]` key to
    // `PropertyKey::Expression` (CLOC12.180); the `computed` flag is set below.
    let key = convert_property_key(key_node)?;

    let kind = if saw_get {
        MethodKind::Get
    } else if saw_set {
        MethodKind::Set
    } else if !is_static
        && !saw_star
        && matches!(&key, PropertyKey::Identifier(id) if id.name == "constructor")
    {
        // `*constructor(){}` is a SyntaxError in real JS — a generator is never a
        // constructor, so a stray `*` guards the `constructor` classification.
        MethodKind::Constructor
    } else {
        MethodKind::Method
    };

    // Params: a lone `formal_parameter` (single param) OR a `formal_parameters`
    // wrapper (two or more). Collect whichever the grammar produced.
    let mut params = Vec::new();
    for n in node_children(node) {
        match n.rule_name.as_str() {
            "formal_parameters" => params.extend(convert_formal_parameters(n)?),
            "formal_parameter" => params.push(convert_formal_parameter(n)?),
            _ => {}
        }
    }

    // Body: the `function_body` node, or an empty block for `m(){}`.
    let body = match node_children(node).into_iter().find(|n| n.rule_name == "function_body") {
        Some(b) => convert_function_body(b)?,
        None => BlockStatement { cv: None, body: vec![] },
    };

    // A computed key `[expr]` bridges to `PropertyKey::Expression` (CLOC12.180),
    // so the `computed` flag tracks exactly that variant.
    let computed = matches!(&key, PropertyKey::Expression(_));
    Ok(MethodDefinition {
        cv: None,
        key,
        kind,
        value: FunctionExpression {
            cv: None,
            id: None,
            params,
            body,
            // `*gen(){}` → a generator method; the emitter re-prints the `*`.
            generator: saw_star,
            is_async: false,
        },
        computed,
        is_static,
    })
}

/// Convert a `private_method_definition` node into a [`MethodDefinition`] whose
/// key is a [`PropertyKey::PrivateName`] — a private class method `#m(){}`
/// (CLOC12.178 PR1).
///
/// Grammar (es2025 `private_method_definition`):
///
/// ```text
///   [ "static" ] PRIVATE_NAME LPAREN [ formal_parameters ] RPAREN LBRACE function_body RBRACE
/// | [ "static" ] "get" PRIVATE_NAME ...          // private getter
/// | [ "static" ] "set" PRIVATE_NAME ...          // private setter
/// | [ "static" ] STAR   PRIVATE_NAME ...          // private generator
/// ```
///
/// This slice models the **plain** method, the **get / set accessor**, and the
/// **generator** (`*#m(){}`) forms (each optionally `static`) — `#m(){}`,
/// `get #x(){}`, `set #x(v){}`, `*#g(){}`. The private *generator* bridges
/// exactly like a public one (CLOC12.182): `saw_star` sets the value's
/// `generator` flag and the emitter reprints the `*`. Only the private *async*
/// form (`async #m(){}`) still DECLINES via `UnsupportedSyntax` (safe
/// WHITESPACE_ONLY fallback), never a mis-emit — `await` is not yet modelled.
///
/// Two shape differences from a public `method_definition`:
/// - the key is a bare `PRIVATE_NAME` token (`#m`), lowered by
///   [`private_name_key`] exactly as a private *field* key is — never a
///   `property_name` node; and
/// - the `static` modifier lives *inside* this node (the grammar's
///   `[ "static" ]`), not on the enclosing `class_element`, so it is read here.
///
/// A private name can never be the `constructor` (`#constructor` is a
/// SyntaxError), so the kind is a plain [`MethodKind::Method`] or the
/// [`MethodKind::Get`] / [`MethodKind::Set`] accessor. Params and body reuse the
/// shared [`convert_formal_parameters`] / [`convert_formal_parameter`] /
/// [`convert_function_body`], mirroring [`convert_method_definition`].
fn convert_private_method_definition(node: &GrammarASTNode) -> Result<MethodDefinition, BridgeError> {
    // Read `static`, the `get` / `set` accessor keyword, and the `*` generator
    // marker (inside this node); decline only the `async` form this slice does
    // not model. All of `static` / `get` / `set` / `*` / `async` precede the
    // PRIVATE_NAME as direct token children (params live under
    // `formal_parameter(s)` *nodes*, so a parameter literally named `get` cannot
    // be confused for the modifier).
    let mut is_static = false;
    let mut saw_get = false;
    let mut saw_set = false;
    let mut saw_star = false;
    let mut decline = false;
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.value.as_str() {
                "static" => is_static = true,
                "get" => saw_get = true,
                "set" => saw_set = true,
                "*" => saw_star = true,
                "async" => decline = true,
                _ => {}
            }
        }
    }
    if decline {
        return Err(unsupported(node));
    }

    let key = private_name_key(node)
        .ok_or_else(|| internal(node, "private_method_definition: missing PRIVATE_NAME"))?;

    // Params: a lone `formal_parameter` OR a `formal_parameters` wrapper.
    let mut params = Vec::new();
    for n in node_children(node) {
        match n.rule_name.as_str() {
            "formal_parameters" => params.extend(convert_formal_parameters(n)?),
            "formal_parameter" => params.push(convert_formal_parameter(n)?),
            _ => {}
        }
    }

    // Body: the `function_body` node, or an empty block for `#m(){}`.
    let body = match node_children(node).into_iter().find(|n| n.rule_name == "function_body") {
        Some(b) => convert_function_body(b)?,
        None => BlockStatement { cv: None, body: vec![] },
    };

    // A private name can never be the `constructor` (`#constructor` is a
    // SyntaxError), so the only kinds are the plain method and the get/set
    // accessors.
    let kind = if saw_get {
        MethodKind::Get
    } else if saw_set {
        MethodKind::Set
    } else {
        MethodKind::Method
    };

    Ok(MethodDefinition {
        cv: None,
        key,
        kind,
        value: FunctionExpression {
            cv: None,
            id: None,
            params,
            body,
            // `*#g(){}` → a private generator method; the emitter reprints the `*`.
            generator: saw_star,
            is_async: false,
        },
        computed: false,
        is_static,
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
fn convert_arrow_function(
    node: &GrammarASTNode,
    is_async: bool,
) -> Result<ArrowFunctionExpression, BridgeError> {
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

    let mut body = body.ok_or_else(|| internal(node, "arrow_function: missing concise_body"))?;

    // The `{` after `=>` ambiguity: the grammar buckets the braces of BOTH a
    // block body `=> {…}` and a parenthesised object body `=> ({…})` as an
    // `object_literal`, so either reaches us as an
    // `ArrowBody::Expression(ObjectExpression)`. Per the ES spec a `{`
    // immediately after `=>` ALWAYS opens a **block** body — an object-literal
    // expression body MUST be parenthesised. We disambiguate by the
    // concise_body's leftmost token: a bare block body leads with `{`, a
    // parenthesised object body leads with `(`.
    if let ArrowBody::Expression(e) = &body {
        if let Expression::ObjectExpression(obj) = &**e {
            let leads_with_brace = children
                .iter()
                .find(|n| n.rule_name == "concise_body")
                .and_then(|n| leftmost_token(n))
                == Some("{");
            if leads_with_brace {
                // Bare `=> {…}` — a BLOCK body per the ES spec.
                if obj.properties.is_empty() {
                    // `=> {}` is an EMPTY block body (CLOC12.184).
                    body = ArrowBody::Block(BlockStatement { cv: None, body: vec![] });
                } else {
                    // `=> {a:1}` — a non-empty block the grammar mis-bucketed as
                    // an object; its contents would need re-parsing as statements,
                    // so DECLINE (safe WHITESPACE_ONLY), never a mis-emit.
                    return Err(unsupported(node));
                }
            }
            // Otherwise the body leads with `(` — a genuine **parenthesised
            // object expression body** `=> ({…})` (CLOC12.185). Keep `body` as an
            // `ArrowBody::Expression(ObjectExpression)`: the emitter re-wraps the
            // object literal in parens so it is never misread as a block.
        }
    }

    Ok(ArrowFunctionExpression {
        cv: None,
        params,
        body,
        is_async,
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
    if let Some(n) = node_children(node).into_iter().next() {
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
        .map(convert_formal_parameter)
        .collect();
    params
}

fn convert_formal_parameter(node: &GrammarASTNode) -> Result<FunctionParam, BridgeError> {
    // formal_parameter = ( NAME | binding_pattern ) [ EQUALS assignment_expression ]
    //                  | ELLIPSIS ( NAME | binding_pattern )
    // Simple NAME identifiers and (CLOC12.190) trailing rest parameters
    // (`...name`) are modelled; a destructuring target is declined.
    if has_token(node, "...") {
        // Rest parameter `...target`. A destructuring rest (`...[a,b]`, `...{x}`)
        // reuses the Phase-3 binding-pattern machinery — decline it rather than
        // mis-model. A simple `...name` bridges to a `FunctionParam::RestElement`.
        for c in &node.children {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == "binding_pattern" {
                    return Err(unsupported(n));
                }
            }
        }
        // The gathered name is the sole non-`...` token in the node.
        let name = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.value != "..." => Some(t.value.clone()),
                _ => None,
            })
            .ok_or_else(|| internal(node, "rest parameter: missing name"))?;
        return Ok(FunctionParam::RestElement(RestElement {
            cv: None,
            argument: Identifier { cv: None, name },
        }));
    }
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == "binding_pattern" {
                return Err(unsupported(n));
            }
        }
    }
    // Default parameter `name = expr` (CLOC12.191). A destructuring target with
    // a default (`{x} = {}`, `[a] = []`) was already declined by the
    // `binding_pattern` guard above, so here the left is always a simple NAME.
    // The right is the sole child *node* — the `assignment_expression` the
    // grammar attaches after `=` — converted through the ordinary expression
    // path so the optimizer folds / renames / inlines it as the live code it is
    // (`function f(a = 1 + 2)` → `function f(a = 3)`).
    if has_token(node, "=") {
        let name = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.value != "=" => Some(t.value.clone()),
                _ => None,
            })
            .ok_or_else(|| internal(node, "default parameter: missing name"))?;
        let right_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .ok_or_else(|| internal(node, "default parameter: missing default expression"))?;
        let right = convert_expression(right_node)?;
        return Ok(FunctionParam::AssignmentPattern(AssignmentPattern {
            cv: None,
            left: Identifier { cv: None, name },
            right,
        }));
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

        // `import.meta` — the module meta-property (CLOC12.168 PR2, gap-169).
        // The grammar emits a dedicated `import_meta` leaf whose children are the
        // three bare tokens `[Token("import"), Token("."), Token("meta")]` (no
        // Node child). It lowers to the atomic `Expression::ImportMeta` leaf, the
        // sibling of `new.target`: the `.meta` is part of the fixed spelling, not
        // a member access, so nothing is walked. Previously fell through to the
        // `other =>` internal-error arm (dropping the file to WHITESPACE_ONLY).
        "import_meta" => convert_import_meta(node),

        // `import(x)` — the dynamic-import call expression (CLOC12.169 PR2,
        // gap-170). The grammar emits a dedicated `dynamic_import` node whose
        // children are `[Token("import"), Token("("), Node(source_expr),
        // Token(")")]` — a single operand (the module-specifier expression)
        // wrapped in the `import( … )` spelling. It lowers to the compound
        // `Expression::ImportExpression` node: unlike `import.meta` (an atomic
        // leaf), this one recurses into its `source` child, so a fold inside the
        // specifier (e.g. `import("a" + "b")` → `import("ab")`) propagates.
        // Previously fell through to the `other =>` internal-error arm, dropping
        // the file to WHITESPACE_ONLY.
        "dynamic_import" => convert_dynamic_import(node),

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
            convert_arrow_function(node, false).map(Expression::ArrowFunctionExpression)
        }

        // Async arrow function (CLOC12.192). The grammar rule
        // `async_arrow_function = "async" arrow_parameters ARROW concise_body`
        // is the plain `arrow_function` shape plus a leading `async` literal, so
        // `convert_arrow_function` handles its children unchanged (the `async`
        // token is not a node) — we just set `is_async`. The AST and emitter
        // already model async arrows; a body that requires `await` still
        // declines separately (that grammar is not parseable yet).
        "async_arrow_function" => {
            convert_arrow_function(node, true).map(Expression::ArrowFunctionExpression)
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
        // Class expression (CLOC12.173 PR2, gap-167). The typed AST gained
        // `Expression::ClassExpression` in PR1; convert the greenfield class
        // instead of declining. `convert_class_expression` itself declines the
        // sub-forms the grammar admits but the typed slice does not yet model
        // (computed `[k]()` keys, `async`/generator methods) via
        // `UnsupportedSyntax`, so an unrepresentable member still drops the
        // whole file to WHITESPACE_ONLY rather than mis-emitting.
        "class_expression" => {
            convert_class_expression(node).map(Expression::ClassExpression)
        }

        "await_expression"
        | "async_function_expression"
        | "async_generator_expression"
        | "tagged_template_expression"
        | "new_target_expression" => Err(unsupported(node)),

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
        // ES2021 logical assignment operators (CLOC12.183). These parse fine but
        // previously fell through to `None`, mapping to an `InternalError` that
        // dropped the whole file to WHITESPACE_ONLY.
        "&&=" => Some(AssignmentOperator::LogicalAndEq),
        "||=" => Some(AssignmentOperator::LogicalOrEq),
        "??=" => Some(AssignmentOperator::NullishCoalescingEq),
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
/// Handles: simple pass-through (no suffix), dot-access, bracket-access, and
/// function-call suffixes, plus the OPTIONAL_CHAIN (`?.`) forms `a?.b` /
/// `a?.[k]` / `a?.()` — each optional link becomes an `OptionalMemberExpression`
/// / `OptionalCallExpression`, and a chain containing any optional link is
/// wrapped once in a `ChainExpression` (CLOC12.171 PR2, closes gap-OptionalChain).
fn convert_optional_chain_expression(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    let nodes = node_children(node);
    if nodes.is_empty() {
        return Err(internal(node, "optional_chain_expression: no children"));
    }

    // Base: the first node child is always the member_expression.
    let mut base = convert_expression(nodes[0])?;

    // Whether any `?.` link appeared in this chain. If so, the whole spine is
    // wrapped once in a `ChainExpression` (the boundary at which the `undefined`
    // short-circuit resolves) before we return.
    let mut saw_optional = false;

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
            ASTNodeOrToken::Token(t) if t.value == "?." => {
                // OPTIONAL_CHAIN — the *following* suffix is optional. The parse
                // tree spells `?.` as its own token followed directly by the
                // suffix (confirmed by parse-tree dump):
                //   `a?.b`    → ?.  NAME            → OptionalMember (dot)
                //   `a?.[k]`  → ?.  [ expr ]        → OptionalMember (computed)
                //   `a?.()`   → ?.  Node(arguments) → OptionalCall
                // A non-optional suffix that follows (e.g. the `.c` in `a?.b.c`)
                // is handled by the ordinary arms below, so only the `?.`-marked
                // link becomes an `Optional*` node.
                saw_optional = true;
                i += 1;
                match children.get(i) {
                    // `?.[` expr `]` — optional computed access.
                    Some(ASTNodeOrToken::Token(t)) if t.value == "[" => {
                        i += 1;
                        while i < children.len() {
                            if let ASTNodeOrToken::Node(key_n) = &children[i] {
                                let key = convert_expression(key_n)?;
                                base = Expression::OptionalMemberExpression(
                                    OptionalMemberExpression {
                                        cv: None,
                                        object: Box::new(base),
                                        property: Box::new(key),
                                        computed: true,
                                    },
                                );
                                i += 1;
                                break;
                            }
                            i += 1;
                        }
                        // Skip the RBRACKET.
                        if let Some(ASTNodeOrToken::Token(t)) = children.get(i) {
                            if t.value == "]" {
                                i += 1;
                            }
                        }
                    }
                    // `?.(` args `)` — optional call.
                    Some(ASTNodeOrToken::Node(arg_n)) if arg_n.rule_name == "arguments" => {
                        let args = convert_arguments(arg_n)?;
                        base = Expression::OptionalCallExpression(OptionalCallExpression {
                            cv: None,
                            callee: Box::new(base),
                            arguments: args,
                        });
                        i += 1;
                    }
                    // `?.` NAME — optional dot access (the bare name token sits
                    // directly after `?.`, with no interposed `.`).
                    Some(ASTNodeOrToken::Token(name_t)) => {
                        let prop_name = name_t.value.clone();
                        i += 1;
                        base = Expression::OptionalMemberExpression(OptionalMemberExpression {
                            cv: None,
                            object: Box::new(base),
                            property: Box::new(Expression::Identifier(Identifier {
                                cv: None,
                                name: prop_name,
                            })),
                            computed: false,
                        });
                    }
                    _ => {
                        return Err(internal(
                            node,
                            "optional_chain_expression: unexpected child after `?.`",
                        ))
                    }
                }
            }
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

    // If any `?.` link appeared, wrap the whole spine once in a
    // `ChainExpression` — the ESTree boundary marker at which the `undefined`
    // short-circuit resolves. A chain with no optional link (a plain
    // `a.b.c` / `f()`) is returned bare, exactly as before.
    if saw_optional {
        base = Expression::ChainExpression(ChainExpression {
            cv: None,
            expression: Box::new(base),
        });
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
// import.meta (CLOC12.168 PR2, gap-169)
// -------------------------------------------------------------------------

/// `import.meta` — the module meta-property. The grammar emits a dedicated
/// `import_meta` leaf whose children are the three bare tokens
/// `[Token("import"), Token("."), Token("meta")]` (no Node child), so — exactly
/// like `new.target` — there is no object / property to fold; the whole thing
/// is one atomic primary. It lowers to the `Expression::ImportMeta` leaf: the
/// `.meta` is part of the fixed spelling, NOT a member access. We take the
/// `import` token's `cv` as the node's provenance (the meta-property is a single
/// conceptual read).
fn convert_import_meta(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    let cv = match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) => t.cv.clone(),
        _ => None,
    };
    Ok(Expression::ImportMeta(ImportMeta { cv }))
}

// -------------------------------------------------------------------------
// import(x) — dynamic import (CLOC12.169 PR2, gap-170)
// -------------------------------------------------------------------------

/// `import(x)` — the dynamic-`import()` call expression. The grammar emits a
/// dedicated `dynamic_import` node with children
/// `[Token("import"), Token("("), Node(source_expr), Token(")")]`: exactly one
/// Node child — the module-specifier expression — flanked by the fixed
/// `import( … )` spelling tokens. We convert that sole child via
/// `convert_expression` and wrap it in the compound
/// `Expression::ImportExpression`.
///
/// Unlike `import.meta` (an atomic *leaf* with no operand), this node has a
/// real `source` operand the downstream passes walk into, so a fold inside the
/// specifier propagates — e.g. `import("a" + "b")` collapses to `import("ab")`.
/// We take the `import` token's `cv` as the node's provenance (the dynamic
/// import is one conceptual construct, mirroring `convert_import_meta`).
///
/// `node_children` skips the three bare tokens, so the specifier is
/// `node_children(node).first()`. If it is somehow absent (a malformed tree),
/// we surface an internal error rather than silently dropping the operand.
fn convert_dynamic_import(node: &GrammarASTNode) -> Result<Expression, BridgeError> {
    let cv = match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) => t.cv.clone(),
        _ => None,
    };
    let source_node = node_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| internal(node, "dynamic_import: no source expression"))?;
    let source = convert_expression(source_node)?;
    Ok(Expression::ImportExpression(ImportExpression {
        cv,
        source: Box::new(source),
    }))
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

    // Every member_expression has at least one Node child (the primary base) —
    // EXCEPT two reserved-word forms whose base is a bare *token*, not a Node:
    //   * the `super`-based forms (`super.x`, `super.m(…)`)         (CLOC12.166)
    //   * the `new.target` meta-property (`[new, ., target]` tokens) (CLOC12.167)
    // Both are handled below, so only reject a genuinely empty node here.
    let super_base =
        matches!(node.children.first(), Some(ASTNodeOrToken::Token(t)) if t.value == "super");
    // `new.target` is the meta-property: three bare tokens `new . target` and
    // NO Node child. (`new X(args)` — the argumented constructor — always has a
    // Node child, its callee, so it stays in the base-init path below; and a
    // lone `new` without a callee never parses.) Distinguishing on
    // `nodes.is_empty()` keeps the two `new` forms cleanly apart.
    let new_target = nodes.is_empty() && has_token(node, "new") && has_token(node, "target");
    if nodes.is_empty() && !super_base && !new_target {
        return Err(internal(node, "member_expression: no children"));
    }

    // Optional chain — Phase 2.
    if has_token(node, "?.") {
        return Err(BridgeError::UnsupportedSyntax {
            rule: "OptionalMemberExpression".to_string(),
            location: loc(node),
        });
    }

    // `new.target` — the meta-property (CLOC12.167 PR2, closes gap-168). The
    // grammar emits it as three bare tokens `[Token("new"), Token("."),
    // Token("target")]` with no Node child, so `new_target` (computed above) is
    // true exactly here. It lowers to the atomic `NewTarget` leaf: the `.` is
    // part of the fixed spelling, NOT a member access, so there is no object /
    // property to fold — the whole thing is one primary. We take the `new`
    // token's `cv` as the node's provenance (the meta-property is a single
    // conceptual read). (The argumented `new X(args)` constructor form has a
    // Node callee and is handled by the base-init below; the two never collide
    // because that form is not `nodes.is_empty()`.)
    if new_target {
        let cv = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t.cv.clone(),
            _ => None,
        };
        return Ok(Expression::NewTarget(NewTarget { cv }));
    }

    // `super` (CLOC12.166 PR2, closes gap-167). Unlike every other primary,
    // `super` is a reserved word that the grammar emits as a *bare token*
    // directly among the member_expression children (not wrapped in a
    // `primary_expression` Node), so `super.m`/`super[k]`/`super(a)` arrive as
    // `[Token("super"), <suffix…>]`. It is handled in two places: a lone
    // `super` (degenerate, no suffix) is returned here; a `super` with a
    // suffix chain becomes the `base` in the suffix-fold below. `super` is
    // syntactically legal only inside a method / derived constructor, but that
    // is enforced upstream — the bridge simply lowers whatever the parser
    // produced. See CLOC12.166 / CLOC02.

    // A bare primary has a SINGLE child overall (just the
    // primary_expression Node, no suffix tokens). We check the full
    // children list, NOT just the Node children: `a.b` has one Node
    // child (`a`) but two suffix tokens (`.` and `b`), so counting
    // Nodes alone would wrongly treat `a.b` as a bare primary and
    // drop the `.b`. (That was the bug this guard previously had.)
    if node.children.len() == 1 {
        // A lone `super` token is a bare `Super` primary (no Node child to
        // pass through — `nodes` is empty here, so we must intercept it).
        if let Some(ASTNodeOrToken::Token(t)) = node.children.first() {
            if t.value == "super" {
                return Ok(Expression::Super(Super { cv: t.cv.clone() }));
            }
        }
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
    } else if let Some(ASTNodeOrToken::Token(t)) = children.first().filter(
        |c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "super"),
    ) {
        // `super` base — a bare token, so the suffix chain (`.NAME` / `[expr]`
        // / call `arguments`) begins at index 1. `super.m`, `super[k]` and
        // `super(a)` all fold from here exactly like an identifier base.
        (Expression::Super(Super { cv: t.cv.clone() }), 1)
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

/// Split a raw regex literal token `/pattern/flags` into `(pattern, flags)`.
///
/// The lexer hands us the ENTIRE literal as one string, delimiters included:
/// `/a\/b/gi` → pattern `a\/b`, flags `gi`. Finding the *closing* delimiter is
/// not a simple "second `/`": a `/` may appear literally inside the pattern
/// when it is backslash-escaped (`\/`) or inside a character class (`[/]`),
/// and neither ends the pattern. We therefore scan character by character:
///
/// ```text
///   state          '/' means…              example
///   ───────────    ─────────────────────   ───────────
///   normal         CLOSE the pattern       /ab/  → close after `ab`
///   after '\'      literal, keep scanning  /a\/b/ → the `\/` is literal
///   inside '[...]' literal, keep scanning  /[/]/  → the `/` is a class member
/// ```
///
/// Per ECMA-262 a `RegularExpressionClass` (`[...]`) is opened by an unescaped
/// `[` and closed by an unescaped `]`; a `/` inside it does NOT terminate the
/// literal (`/[/]/` is a valid regex matching a single slash). A `\` escapes
/// the very next character in either state. We honour both so the delimiter we
/// pick is the true closing `/`.
///
/// Returns `None` if the token is malformed (does not start with `/`, or has no
/// closing `/`) — the caller turns that into a bridge `InternalError` rather
/// than silently mis-splitting.
fn split_regex_literal(raw: &str) -> Option<(String, String)> {
    let bytes = raw.as_bytes();
    // Must open with a delimiter; anything else is not a regex literal token.
    if bytes.first() != Some(&b'/') {
        return None;
    }

    let mut i = 1; // skip the opening '/'
    let mut in_class = false; // inside a `[...]` character class?
    let mut close: Option<usize> = None;
    while i < bytes.len() {
        match bytes[i] {
            // A backslash escapes the next byte in BOTH states. Skip both so an
            // escaped delimiter (`\/`) or escaped bracket (`\[`, `\]`) is inert.
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' if !in_class => in_class = true,
            b']' if in_class => in_class = false,
            // The closing delimiter: an unescaped `/` OUTSIDE a character class.
            b'/' if !in_class => {
                close = Some(i);
                break;
            }
            _ => {}
        }
        i += 1;
    }

    let close = close?;
    // `pattern` is between the delimiters; `flags` is everything after the
    // closing `/`. Slicing on ASCII `/`/`\`/`[`/`]` byte boundaries is safe:
    // every index we key on is a single-byte ASCII position, and any multi-byte
    // UTF-8 sequence in the pattern is copied through untouched.
    let pattern = raw[1..close].to_string();
    let flags = raw[close + 1..].to_string();
    Some((pattern, flags))
}

/// Convert a single-token primary expression.
///
/// NUMBER/STRING/NAME are encoded in `t.type_` (not `t.type_name`).
/// BIGINT has `type_ = TokenType::Name` and `type_name = Some("BIGINT")`.
/// REGEX has `type_ = TokenType::Name` and `type_name = Some("REGEX")`.
fn convert_primary_token(t: &Token, ctx: &GrammarASTNode) -> Result<Expression, BridgeError> {
    // Value-based checks first (keywords: this, true, false, null, undefined).
    //
    // These MUST be gated on the token TYPE: only an identifier-like token
    // (`Name`/`Keyword`) may be reinterpreted as one of these keyword primaries.
    // A `String`/`Number` *literal* token whose text happens to equal a keyword
    // — a string whose content is `this`/`true`/`false`/`null`/`undefined`, or
    // the value `"true"` in source — is NOT that keyword and must flow to the
    // type-discriminant arms below (`TokenType::String` → `StringLiteral`, etc.).
    // Without this gate `f("true")` mis-encodes to `f(true)` and `f("this")` to
    // `f(this)` — a hard miscompile: a string argument silently becomes a
    // boolean / the `this` value. (The reference Closure Compiler keeps the
    // string.) Matching the documented design: "NUMBER/STRING/NAME are encoded
    // in `t.type_`" — the value match is only for the keyword primaries.
    if matches!(t.type_, TokenType::Name | TokenType::Keyword) {
        match t.value.as_str() {
            "this" => return Ok(Expression::ThisExpression(ThisExpression { cv: t.cv.clone() })),
            "null" => return Ok(Expression::NullLiteral(NullLiteral { cv: t.cv.clone() })),
            "undefined" => {
                return Ok(Expression::UndefinedLiteral(UndefinedLiteral { cv: t.cv.clone() }))
            }
            "true" => {
                return Ok(Expression::BooleanLiteral(BooleanLiteral {
                    cv: t.cv.clone(),
                    value: true,
                }))
            }
            "false" => {
                return Ok(Expression::BooleanLiteral(BooleanLiteral {
                    cv: t.cv.clone(),
                    value: false,
                }))
            }
            _ => {}
        }
    }

    // BIGINT: type_ == TokenType::Name but type_name == Some("BIGINT").
    if t.type_name.as_deref() == Some("BIGINT") {
        let raw = t.value.clone();
        let value = raw.trim_end_matches('n').to_string();
        return Ok(Expression::BigIntLiteral(BigIntLiteral { cv: t.cv.clone(), value, raw }));
    }

    // REGEX: type_ == TokenType::Name but type_name == Some("REGEX"). The lexer
    // emits the ENTIRE literal as one token whose `value` is `/pat/flags`
    // (escapes and char-class contents preserved verbatim, e.g. `/a\/b/gi`).
    // We split it into `pattern` and `flags` around the CLOSING `/` so the
    // emitter can round-trip it as a `RegExpLiteral`. Without this arm the
    // catch-all below would mis-encode the whole literal as an `Identifier`
    // named `/pat/flags` (gap-RegExpAsIdentifier), which the emitter then
    // prints as raw text — a latent miscompile if the identifier were ever
    // renamed or folded.
    if t.type_name.as_deref() == Some("REGEX") {
        let (pattern, flags) = split_regex_literal(&t.value).ok_or_else(|| {
            BridgeError::InternalError {
                msg: format!("malformed regex literal token '{}'", t.value),
                rule: ctx.rule_name.clone(),
            }
        })?;
        // Defence-in-depth (debug builds): the split must leave the `pattern`
        // free of raw line terminators (ECMA-262 forbids them in a regex
        // literal) and the `flags` restricted to the valid ES set `dgimsuy`.
        // The lexer already enforces both; these asserts catch any future
        // lexer/grammar drift before a malformed literal reaches the emitter.
        debug_assert!(
            !pattern.contains('\n') && !pattern.contains('\r'),
            "regex pattern must not contain a raw line terminator: {pattern:?}"
        );
        debug_assert!(
            flags.chars().all(|c| "dgimsuy".contains(c)),
            "regex flags must be a subset of [dgimsuy]: {flags:?}"
        );
        return Ok(Expression::RegExpLiteral(RegExpLiteral {
            cv: t.cv.clone(),
            pattern,
            flags,
        }));
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
        if n.rule_name.as_str() == "property_definition" {
            // A `property_definition` is either a normal member (`k: v`,
            // shorthand `{x}`, getter/setter) or an **object spread** `...expr`
            // (ES2018). Dumping the parse tree shows the spread form nests one
            // level deeper than the call/array spread: the `property_definition`
            // holds a single `object_spread_property` Node child whose own
            // children are `[ Token("..."), Node(assignment_expression) ]`.
            // (The call/array spread's ELLIPSIS sits directly under
            // `spread_element` — a different rule.) So we detect the spread by
            // that inner rule name, not `has_token` on `property_definition`.
            // CLOC12.170 PR2, closes gap-SpreadProperty.
            let spread = node_children(n)
                .into_iter()
                .find(|c| c.rule_name == "object_spread_property");
            if let Some(spread_node) = spread {
                // `node_children` strips the ELLIPSIS token, leaving the single
                // `assignment_expression`. Reuse `SpreadElement` (the same node
                // the call/array spread uses) so it prints via `emit_object_spread`.
                let arg_n = node_children(spread_node).into_iter().next().ok_or_else(
                    || internal(spread_node, "object spread: no argument expression"),
                )?;
                let argument = convert_expression(arg_n)?;
                properties.push(ObjectMember::Spread(SpreadElement {
                    cv: None,
                    argument: Box::new(argument),
                }));
            } else {
                properties.push(ObjectMember::Property(convert_property_definition(n)?));
            }
        }
    }
    Ok(Expression::ObjectExpression(ObjectExpression { cv: None, properties }))
}

fn convert_property_definition(node: &GrammarASTNode) -> Result<Property, BridgeError> {
    // property_definition = property_name COLON assignment_expression
    //                     | NAME  (shorthand)
    //                     | method_definition  (unsupported Phase 2)
    //
    // The ELLIPSIS spread form `...expr` is NOT handled here — `convert_object_literal`
    // detects it (`has_token`) and builds `ObjectMember::Spread` directly, since a
    // spread is a *member* but not a `Property`. This fn only sees plain members.
    let nodes = node_children(node);
    if nodes.len() == 2 {
        // property_name : value
        let key_n = nodes[0];
        let val_n = nodes[1];
        let key = convert_property_key(key_n)?;
        let value = convert_expression(val_n)?;
        // A computed key `[expr]` bridges to `PropertyKey::Expression`
        // (CLOC12.180); the `computed` flag tracks exactly that variant.
        let computed = matches!(&key, PropertyKey::Expression(_));
        return Ok(Property {
            cv: None,
            key,
            value: Box::new(value),
            kind: PropertyKind::Init,
            shorthand: false,
            computed,
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
    // A **computed** key `[expr]` — the `property_name` wraps the key expression
    // between `[` and `]`. Convert the inner expression to
    // `PropertyKey::Expression`; the emitter re-brackets it (CLOC12.180). The
    // inner node is an `assignment_expression` (the same node a field
    // initializer uses), routed through the shared `convert_expression`, so any
    // unmodelled key expression DECLINES (safe WHITESPACE_ONLY fallback) rather
    // than mis-emit.
    if has_token(node, "[") {
        let inner = node_children(node)
            .into_iter()
            .next()
            .ok_or_else(|| internal(node, "computed key: missing key expression"))?;
        return Ok(PropertyKey::Expression(Box::new(convert_expression(inner)?)));
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
            // Legacy octal escape `\NNN` (ECMAScript Annex B.1.2) — one to three
            // octal digits denoting a code unit in `0..=255`. `\0`→NUL,
            // `\101`→'A', `\012`→'\n'. A leading digit `0`–`3` admits up to
            // THREE octal digits; a leading `4`–`7` admits at most TWO, so the
            // decoded value never exceeds `0o377` (= 255) — matching the grammar
            // productions (ZeroToThree may take two trailing octal digits,
            // FourToSeven only one). The reference Closure Compiler decodes these
            // to the raw character (`"\101"` → `"A"`), and closurec must
            // round-trip the identical value; previously `\1`–`\7` fell through
            // to the identity arm and `\NNN` survived undecoded — a miscompile
            // (the string value was wrong, not just the spelling). Legacy octal
            // is forbidden in strict-mode source, but sloppy string literals
            // permit it, so the fold set must handle it.
            Some(d @ '0'..='7') => {
                // SAFETY of unwrap: `d` is a validated octal digit `0`–`7`.
                let mut value = d.to_digit(8).expect("octal digit");
                // Closure reads UP TO THREE octal digits regardless of the
                // leading digit (value 0..=0o777=511), NOT the ECMAScript Annex
                // B two-digit cap for a leading 4-7. Byte-identity requires we
                // match Closure: `\401`->U+0101, `\777`->U+01FF (oracle-verified).
                for _ in 0..2 {
                    match chars.peek() {
                        Some(&next @ '0'..='7') => {
                            value = value * 8 + next.to_digit(8).expect("octal digit");
                            chars.next();
                        }
                        _ => break,
                    }
                }
                // `value` is at most 0o777 = 511, always a valid Unicode scalar.
                if let Some(ch) = char::from_u32(value) {
                    result.push(ch);
                }
            }
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

    fn bridge(src: &str) -> Result<Program, BridgeError> {
        let node = parse_javascript_typed(src, DEFAULT_ES_VERSION).expect("parse failed");
        grammar_to_program(&node, DEFAULT_ES_VERSION)
    }

    /// Pull the sole `ArrowFunctionExpression` out of `x = <arrow>;`.
    fn arrow_of(src: &str) -> ArrowFunctionExpression {
        let p = bridge_ok(src);
        match first_expr(&p) {
            Expression::AssignmentExpression(a) => match &*a.right {
                Expression::ArrowFunctionExpression(f) => f.clone(),
                other => panic!("expected ArrowFunctionExpression RHS, got {other:?}"),
            },
            other => panic!("expected AssignmentExpression, got {other:?}"),
        }
    }

    #[test]
    fn arrow_empty_block_body_bridges() {
        // `() => {}` — the grammar buckets the bare `{}` as an empty
        // object_literal, but per the ES spec `=> {}` is an EMPTY BLOCK body.
        // CLOC12.184 reinterprets it as `ArrowBody::Block` with no statements,
        // instead of declining to WHITESPACE_ONLY.
        let f = arrow_of("x = () => {};");
        assert!(f.params.is_empty());
        match &f.body {
            ArrowBody::Block(b) => assert!(b.body.is_empty(), "expected empty block"),
            other => panic!("expected an empty block body, got {other:?}"),
        }
    }

    #[test]
    fn arrow_paren_object_body_bridges() {
        // `() => ({})` / `() => ({a:1})` — a parenthesised object-literal
        // EXPRESSION body (leads with `(`). Distinct from the bare block `=> {}`;
        // bridges to `ArrowBody::Expression(ObjectExpression)` (CLOC12.185). The
        // emitter re-wraps the object in parens so it is never misread as a block.
        for src in ["x = () => ({});", "x = () => ({a:1});"] {
            let f = arrow_of(src);
            assert!(f.params.is_empty());
            match &f.body {
                ArrowBody::Expression(e) => assert!(
                    matches!(&**e, Expression::ObjectExpression(_)),
                    "expected an object-expression body for {src}"
                ),
                other => panic!("expected an object-expression body for {src}, got {other:?}"),
            }
        }
    }

    #[test]
    fn arrow_nonempty_brace_body_still_declines() {
        // `() => {a:1}` — a non-empty `{…}` the grammar mis-buckets as an object
        // literal. Its contents would need re-parsing as statements, so it stays
        // declined (a later slice), never a mis-emit.
        assert!(matches!(
            bridge("x = () => {a:1};"),
            Err(BridgeError::UnsupportedSyntax { .. })
        ));
    }

    fn bridge_ok(src: &str) -> Program {
        bridge(src).unwrap_or_else(|e| panic!("bridge failed for {:?}: {e}", src))
    }

    /// Pull the sole `ImportDeclaration` out of a single-item program.
    fn import_of(src: &str) -> ImportDeclaration {
        let p = bridge_ok(src);
        match p.body.first() {
            Some(ProgramItem::Declaration(Declaration::ImportDeclaration(i))) => i.clone(),
            other => panic!("expected an ImportDeclaration for {src}, got {other:?}"),
        }
    }

    /// Pull the sole `FunctionDeclaration` out of a single-item program.
    fn fn_of(src: &str) -> FunctionDeclaration {
        let p = bridge_ok(src);
        match p.body.first() {
            Some(ProgramItem::Declaration(Declaration::FunctionDeclaration(f))) => f.clone(),
            other => panic!("expected a FunctionDeclaration for {src}, got {other:?}"),
        }
    }

    #[test]
    fn rest_parameter_bridges() {
        // `function f(...args){}` — CLOC12.190 PR2. The lone `...args` bridges to
        // a `FunctionParam::RestElement` binding the name `args`, instead of the
        // whole file declining to WHITESPACE_ONLY.
        let f = fn_of("function f(...args){}");
        assert_eq!(f.params.len(), 1);
        match &f.params[0] {
            FunctionParam::RestElement(re) => assert_eq!(re.argument.name, "args"),
            other => panic!("expected a RestElement param, got {other:?}"),
        }
    }

    #[test]
    fn fixed_then_rest_parameter_bridges() {
        // `function f(a, ...rest){}` — the fixed `a` stays an Identifier param and
        // the trailing `...rest` bridges to a RestElement, in order.
        let f = fn_of("function f(a, ...rest){}");
        assert_eq!(f.params.len(), 2);
        match &f.params[0] {
            FunctionParam::Identifier(id) => assert_eq!(id.name, "a"),
            other => panic!("expected Identifier for param 0, got {other:?}"),
        }
        match &f.params[1] {
            FunctionParam::RestElement(re) => assert_eq!(re.argument.name, "rest"),
            other => panic!("expected RestElement for param 1, got {other:?}"),
        }
    }

    #[test]
    fn rest_destructuring_param_declines_gracefully() {
        // `function f(...[a, b]){}` — a destructuring rest target is Phase 3, so
        // the bridge declines (the whole program falls back to WHITESPACE_ONLY)
        // rather than mis-modelling it. A decline is an Err, never a panic.
        assert!(
            bridge("function f(...[a, b]){}").is_err(),
            "destructuring rest param should decline, not bridge"
        );
    }

    #[test]
    fn default_parameter_bridges() {
        // `function f(a = 1){}` — CLOC12.191 PR2. The `a = 1` bridges to a
        // `FunctionParam::AssignmentPattern` binding `a` with a numeric-literal
        // default, instead of the whole file declining to WHITESPACE_ONLY.
        let f = fn_of("function f(a = 1){}");
        assert_eq!(f.params.len(), 1);
        match &f.params[0] {
            FunctionParam::AssignmentPattern(ap) => {
                assert_eq!(ap.left.name, "a");
                match &ap.right {
                    Expression::NumericLiteral(n) => assert_eq!(n.value, 1.0),
                    other => panic!("expected a numeric-literal default, got {other:?}"),
                }
            }
            other => panic!("expected an AssignmentPattern param, got {other:?}"),
        }
    }

    #[test]
    fn fixed_then_default_parameter_bridges() {
        // `function f(a, b = 2){}` — the fixed `a` stays an Identifier param and
        // `b = 2` bridges to an AssignmentPattern, in order.
        let f = fn_of("function f(a, b = 2){}");
        assert_eq!(f.params.len(), 2);
        match &f.params[0] {
            FunctionParam::Identifier(id) => assert_eq!(id.name, "a"),
            other => panic!("expected Identifier for param 0, got {other:?}"),
        }
        match &f.params[1] {
            FunctionParam::AssignmentPattern(ap) => assert_eq!(ap.left.name, "b"),
            other => panic!("expected AssignmentPattern for param 1, got {other:?}"),
        }
    }

    #[test]
    fn default_parameter_expression_bridges_unfolded() {
        // `function f(a = 1 + 2){}` — the default's `right` is a full expression
        // (a `BinaryExpression`), NOT pre-folded: the bridge only models the
        // shape; constant-fold does the folding downstream. This is the whole
        // point of `right` being an Expression rather than a literal.
        let f = fn_of("function f(a = 1 + 2){}");
        match &f.params[0] {
            FunctionParam::AssignmentPattern(ap) => {
                assert_eq!(ap.left.name, "a");
                assert!(
                    matches!(ap.right, Expression::BinaryExpression(_)),
                    "default `1 + 2` must bridge as an (unfolded) BinaryExpression, got {:?}",
                    ap.right
                );
            }
            other => panic!("expected an AssignmentPattern param, got {other:?}"),
        }
    }

    #[test]
    fn destructuring_default_param_declines_gracefully() {
        // `function f({x} = {}){}` — a destructuring target WITH a default reuses
        // the Phase-3 binding-pattern machinery, so the bridge declines (falls
        // back to WHITESPACE_ONLY) via the `binding_pattern` guard rather than
        // mis-modelling it. A decline is an Err, never a panic.
        assert!(
            bridge("function f({x} = {}){}").is_err(),
            "destructuring default param should decline, not bridge"
        );
    }

    #[test]
    fn bridge_side_effect_import() {
        // `import "y";` — no specifiers, source "y".
        let i = import_of("import \"y\";");
        assert!(i.specifiers.is_empty());
        assert_eq!(i.source.value, "y");
    }

    #[test]
    fn bridge_default_import() {
        // `import x from "y";` — one Default specifier.
        let i = import_of("import x from \"y\";");
        assert_eq!(i.source.value, "y");
        assert_eq!(i.specifiers.len(), 1);
        match &i.specifiers[0] {
            ImportSpecifier::Default(id) => assert_eq!(id.name, "x"),
            other => panic!("expected Default, got {other:?}"),
        }
    }

    #[test]
    fn bridge_namespace_import() {
        // `import * as ns from "y";` — one Namespace specifier.
        let i = import_of("import * as ns from \"y\";");
        match &i.specifiers[..] {
            [ImportSpecifier::Namespace(id)] => assert_eq!(id.name, "ns"),
            other => panic!("expected [Namespace], got {other:?}"),
        }
    }

    #[test]
    fn bridge_named_imports_plain_and_aliased() {
        // `import {a, b as c} from "y";` — `a` binds a→a, `b as c` binds b→c.
        let i = import_of("import {a, b as c} from \"y\";");
        match &i.specifiers[..] {
            [
                ImportSpecifier::Named { imported: i0, local: l0 },
                ImportSpecifier::Named { imported: i1, local: l1 },
            ] => {
                assert_eq!((i0.name.as_str(), l0.name.as_str()), ("a", "a"));
                assert_eq!((i1.name.as_str(), l1.name.as_str()), ("b", "c"));
            }
            other => panic!("expected two Named specifiers, got {other:?}"),
        }
    }

    #[test]
    fn bridge_default_plus_named_import() {
        // `import x, {a} from "y";` — Default then Named.
        let i = import_of("import x, {a} from \"y\";");
        match &i.specifiers[..] {
            [
                ImportSpecifier::Default(d),
                ImportSpecifier::Named { imported, local },
            ] => {
                assert_eq!(d.name, "x");
                assert_eq!((imported.name.as_str(), local.name.as_str()), ("a", "a"));
            }
            other => panic!("expected [Default, Named], got {other:?}"),
        }
    }

    #[test]
    fn bridge_default_plus_namespace_import_declines() {
        // `import x, * as ns from "y";` is a grammar gap — the parser rejects
        // the default+namespace combination at the parse layer (before the
        // bridge ever runs), so the whole file declines rather than
        // mis-bridging.
        assert!(parse_javascript_typed("import x, * as ns from \"y\";", DEFAULT_ES_VERSION).is_err());
    }

    /// Pull the sole `Declaration::Export*` out of a single-item program.
    fn export_of(src: &str) -> Declaration {
        let p = bridge_ok(src);
        match p.body.first() {
            Some(ProgramItem::Declaration(
                d @ (Declaration::ExportNamedDeclaration(_)
                | Declaration::ExportDefaultDeclaration(_)
                | Declaration::ExportAllDeclaration(_)),
            )) => d.clone(),
            other => panic!("expected an Export* declaration for {src}, got {other:?}"),
        }
    }

    #[test]
    fn bridge_export_named_plain_and_aliased() {
        // `export {a, b as c};` — `a` → local=exported=a, `b as c` → local=b,
        // exported=c; no inner declaration, no source.
        match export_of("export {a, b as c};") {
            Declaration::ExportNamedDeclaration(e) => {
                assert!(e.declaration.is_none());
                assert!(e.source.is_none());
                match &e.specifiers[..] {
                    [s0, s1] => {
                        assert_eq!((s0.local.name.as_str(), s0.exported.name.as_str()), ("a", "a"));
                        assert_eq!((s1.local.name.as_str(), s1.exported.name.as_str()), ("b", "c"));
                    }
                    other => panic!("expected two specifiers, got {other:?}"),
                }
            }
            other => panic!("expected ExportNamedDeclaration, got {other:?}"),
        }
    }

    #[test]
    fn bridge_export_named_reexport() {
        // `export {a} from "y";` — carries a re-export source.
        match export_of("export {a} from \"y\";") {
            Declaration::ExportNamedDeclaration(e) => {
                assert_eq!(e.specifiers.len(), 1);
                assert_eq!(e.source.as_ref().map(|s| s.value.as_str()), Some("y"));
            }
            other => panic!("expected ExportNamedDeclaration, got {other:?}"),
        }
    }

    #[test]
    fn bridge_export_all() {
        // `export * from "y";` — bare re-export-all, no namespace binding.
        match export_of("export * from \"y\";") {
            Declaration::ExportAllDeclaration(e) => {
                assert!(e.exported.is_none());
                assert_eq!(e.source.value, "y");
            }
            other => panic!("expected ExportAllDeclaration, got {other:?}"),
        }
    }

    #[test]
    fn bridge_export_default_expression() {
        // `export default 1;` — an expression operand.
        match export_of("export default 1;") {
            Declaration::ExportDefaultDeclaration(e) => assert!(matches!(
                e.declaration,
                ExportDefaultKind::Expression(_)
            )),
            other => panic!("expected ExportDefaultDeclaration, got {other:?}"),
        }
    }

    #[test]
    fn bridge_export_default_function_and_class() {
        match export_of("export default function f(){}") {
            Declaration::ExportDefaultDeclaration(e) => assert!(matches!(
                e.declaration,
                ExportDefaultKind::FunctionDeclaration(_)
            )),
            other => panic!("expected ExportDefaultDeclaration(fn), got {other:?}"),
        }
        match export_of("export default class C{}") {
            Declaration::ExportDefaultDeclaration(e) => assert!(matches!(
                e.declaration,
                ExportDefaultKind::ClassDeclaration(_)
            )),
            other => panic!("expected ExportDefaultDeclaration(class), got {other:?}"),
        }
    }

    #[test]
    fn bridge_export_declaration_const_var_function_class() {
        // `export const x = 1;` / `export var v = 1;` / `export function f(){}` /
        // `export class C {}` — each wraps its inner declaration.
        for (src, want) in [
            ("export const x = 1;", "var"),
            ("export var v = 1;", "var"),
            ("export function f(){}", "fn"),
            ("export class C {}", "class"),
        ] {
            match export_of(src) {
                Declaration::ExportNamedDeclaration(e) => {
                    assert!(e.specifiers.is_empty());
                    assert!(e.source.is_none());
                    let got = match e.declaration.as_deref() {
                        Some(Declaration::VariableDeclaration(_)) => "var",
                        Some(Declaration::FunctionDeclaration(_)) => "fn",
                        Some(Declaration::ClassDeclaration(_)) => "class",
                        other => panic!("unexpected inner decl for {src}: {other:?}"),
                    };
                    assert_eq!(got, want, "for {src}");
                }
                other => panic!("expected ExportNamedDeclaration for {src}, got {other:?}"),
            }
        }
    }

    #[test]
    fn bridge_export_star_as_namespace_declines() {
        // `export * as ns from "y";` is a grammar gap — rejected at the parse
        // layer, so the file declines rather than mis-bridging.
        assert!(parse_javascript_typed("export * as ns from \"y\";", DEFAULT_ES_VERSION).is_err());
    }

    /// Pull the `AssignmentExpression` operator out of `<lhs> <op> <rhs>;`.
    fn assign_op_of(src: &str) -> AssignmentOperator {
        let p = bridge_ok(src);
        match first_expr(&p) {
            Expression::AssignmentExpression(a) => a.operator,
            other => panic!("expected AssignmentExpression, got {other:?}"),
        }
    }

    #[test]
    fn logical_assignment_operators_bridge() {
        // ES2021 `&&=` / `||=` / `??=` parse fine but previously mapped to an
        // InternalError ("unknown assignment operator"), dropping the file to
        // WHITESPACE_ONLY. They now bridge to their own operator variants
        // (CLOC12.183).
        assert_eq!(assign_op_of("a &&= b;"), AssignmentOperator::LogicalAndEq);
        assert_eq!(assign_op_of("a ||= b;"), AssignmentOperator::LogicalOrEq);
        assert_eq!(assign_op_of("a ??= b;"), AssignmentOperator::NullishCoalescingEq);
        // A neighbouring bitwise `&=` must still map to its own (distinct) variant.
        assert_eq!(assign_op_of("a &= b;"), AssignmentOperator::BitAndEq);
    }

    /// Pull the `RegExpLiteral` out of `x = <regex>;` so the regex tests can
    /// assert on `(pattern, flags)` directly.
    fn regex_of(src: &str) -> RegExpLiteral {
        let p = bridge_ok(src);
        match first_expr(&p) {
            Expression::AssignmentExpression(a) => match &*a.right {
                Expression::RegExpLiteral(r) => r.clone(),
                other => panic!("expected RegExpLiteral RHS, got {other:?}"),
            },
            other => panic!("expected AssignmentExpression, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // ClassExpression bridging (CLOC12.173 PR2, gap-167)
    // -----------------------------------------------------------------

    /// Pull the `ClassExpression` out of `x = <class …>;`.
    fn class_of(src: &str) -> ClassExpression {
        let p = bridge_ok(src);
        match first_expr(&p) {
            Expression::AssignmentExpression(a) => match &*a.right {
                Expression::ClassExpression(c) => c.clone(),
                other => panic!("expected ClassExpression RHS, got {other:?}"),
            },
            other => panic!("expected AssignmentExpression, got {other:?}"),
        }
    }

    #[test]
    fn class_empty_anonymous() {
        let c = class_of("x = class {};");
        assert!(c.id.is_none());
        assert!(c.super_class.is_none());
        assert!(c.body.is_empty());
    }

    #[test]
    fn class_named() {
        let c = class_of("x = class C {};");
        assert_eq!(c.id.as_ref().map(|i| i.name.as_str()), Some("C"));
        assert!(c.super_class.is_none());
    }

    #[test]
    fn class_extends_identifier() {
        // `extends B` — the heritage operand is a bare NAME token.
        let c = class_of("x = class C extends B {};");
        match c.super_class.as_deref() {
            Some(Expression::Identifier(id)) => assert_eq!(id.name, "B"),
            other => panic!("expected Identifier super_class, got {other:?}"),
        }
    }

    #[test]
    fn class_extends_member() {
        // `extends ns.B` — the heritage operand is a member-expression NODE.
        let c = class_of("x = class extends ns.B {};");
        match c.super_class.as_deref() {
            Some(Expression::MemberExpression(m)) => {
                assert!(matches!(&*m.object, Expression::Identifier(i) if i.name == "ns"));
            }
            other => panic!("expected MemberExpression super_class, got {other:?}"),
        }
    }

    #[test]
    fn class_method() {
        let c = class_of("x = class { m(a,b){return a} };");
        assert_eq!(c.body.len(), 1);
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.kind, MethodKind::Method);
        assert!(!m.is_static);
        assert!(matches!(&m.key, PropertyKey::Identifier(id) if id.name == "m"));
        assert_eq!(m.value.params.len(), 2);
    }

    #[test]
    fn class_single_param_method() {
        // A single param parses as a direct `formal_parameter` (no wrapper);
        // it must still be collected.
        let c = class_of("x = class { m(v){return v} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.value.params.len(), 1);
    }

    #[test]
    fn class_static_method() {
        let c = class_of("x = class { static m(){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(m.is_static);
        assert_eq!(m.kind, MethodKind::Method);
    }

    #[test]
    fn class_getter() {
        let c = class_of("x = class { get g(){return 1} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.kind, MethodKind::Get);
        assert!(matches!(&m.key, PropertyKey::Identifier(id) if id.name == "g"));
    }

    #[test]
    fn class_setter() {
        let c = class_of("x = class { set s(v){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.kind, MethodKind::Set);
        assert_eq!(m.value.params.len(), 1);
    }

    #[test]
    fn class_method_named_get_is_plain_method() {
        // `get(){}` — a method whose NAME is `get`, NOT a getter. The grammar
        // puts the `property_name` node first (no leading accessor token), so
        // the bridge must classify it as an ordinary method.
        let c = class_of("x = class { get(){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.kind, MethodKind::Method);
        assert!(matches!(&m.key, PropertyKey::Identifier(id) if id.name == "get"));
    }

    #[test]
    fn class_constructor() {
        let c = class_of("x = class { constructor(a){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.kind, MethodKind::Constructor);
        assert!(matches!(&m.key, PropertyKey::Identifier(id) if id.name == "constructor"));
    }

    #[test]
    fn class_static_constructor_is_plain_method() {
        // A `static constructor(){}` is NOT the special constructor — only a
        // non-static `constructor` member is. (Legal JS: a static method may be
        // named `constructor`.)
        let c = class_of("x = class { static constructor(){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(m.is_static);
        assert_eq!(m.kind, MethodKind::Method);
    }

    #[test]
    fn class_computed_method_key() {
        // `[k](){}` — a computed method key bridges to `PropertyKey::Expression`
        // and sets `computed: true` (CLOC12.180).
        let c = class_of("x = class { [k](){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(m.computed);
        assert!(matches!(&m.key, PropertyKey::Expression(e)
            if matches!(&**e, Expression::Identifier(id) if id.name == "k")));
    }

    #[test]
    fn class_generator_method_bridges() {
        // `*gen(){}` — a generator method bridges (CLOC12.181): plain method
        // `kind`, and the value's `generator` flag is set so the emitter reprints
        // the `*`. `yield` in the body is a modelled `YieldExpression`.
        let c = class_of("x = class { *gen(){ yield 1 } };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert_eq!(m.kind, MethodKind::Method);
        assert!(m.value.generator);
        assert!(!m.value.is_async);
        assert!(matches!(&m.key, PropertyKey::Identifier(id) if id.name == "gen"));
    }

    #[test]
    fn class_static_generator_method_bridges() {
        // `static *gen(){}` — the `static` modifier lives on the enclosing
        // `class_element`; the `*` still sets the generator flag.
        let c = class_of("x = class { static *gen(){} };");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(m.is_static);
        assert!(m.value.generator);
        assert_eq!(m.kind, MethodKind::Method);
    }

    #[test]
    fn class_async_method_declines() {
        assert!(matches!(
            bridge("x = class { async am(){} };"),
            Err(BridgeError::UnsupportedSyntax { .. })
        ));
    }

    // -----------------------------------------------------------------
    // ClassMember::Field bridging (CLOC12.175 PR2)
    // -----------------------------------------------------------------

    /// Pull the sole `ClassMember::Field` out of `x = class { <field> };`.
    fn field_of(src: &str) -> PropertyDefinition {
        let c = class_of(src);
        assert_eq!(c.body.len(), 1, "expected exactly one member");
        match &c.body[0] {
            ClassMember::Field(f) => f.clone(),
            other => panic!("expected a field member, got {other:?}"),
        }
    }

    #[test]
    fn class_field_with_initializer() {
        // `x = 1;` — an identifier key with a numeric initializer.
        let f = field_of("y = class { x = 1; };");
        assert!(!f.is_static);
        assert!(!f.computed);
        assert!(matches!(&f.key, PropertyKey::Identifier(id) if id.name == "x"));
        match &f.value {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 1.0),
            other => panic!("expected a numeric initializer, got {other:?}"),
        }
    }

    #[test]
    fn class_bare_field_has_no_value() {
        // `y;` — a bare field with no initializer maps to `value: None`.
        let f = field_of("z = class { y; };");
        assert!(matches!(&f.key, PropertyKey::Identifier(id) if id.name == "y"));
        assert!(f.value.is_none());
    }

    #[test]
    fn class_static_field() {
        // `static z = 2;` — the field's OWN `static` token (inside
        // `class_field_declaration`, not on the `class_element`).
        let f = field_of("w = class { static z = 2; };");
        assert!(f.is_static);
        assert!(matches!(&f.key, PropertyKey::Identifier(id) if id.name == "z"));
        match &f.value {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 2.0),
            other => panic!("expected a numeric initializer, got {other:?}"),
        }
    }

    #[test]
    fn class_string_key_field() {
        // A quoted key decodes to a `StringLiteral` key (the emitter later
        // decides quote-vs-bare); the initializer is an identifier reference.
        let f = field_of("w = class { \"a-b\" = q; };");
        match &f.key {
            PropertyKey::StringLiteral(s) => assert_eq!(s.value, "a-b"),
            other => panic!("expected a string key, got {other:?}"),
        }
        assert!(matches!(&f.value, Some(Expression::Identifier(id)) if id.name == "q"));
    }

    #[test]
    fn class_field_and_method_interleave() {
        // A field and a method coexist in one body, in source order.
        let c = class_of("w = class { x = 1; m(){} };");
        assert_eq!(c.body.len(), 2);
        assert!(matches!(&c.body[0], ClassMember::Field(f)
            if matches!(&f.key, PropertyKey::Identifier(id) if id.name == "x")));
        assert!(matches!(&c.body[1], ClassMember::Method(m)
            if matches!(&m.key, PropertyKey::Identifier(id) if id.name == "m")));
    }

    #[test]
    fn class_computed_field_key() {
        // `[k] = v;` — a computed field key bridges to `PropertyKey::Expression`
        // with `computed: true`, and the initializer is preserved (CLOC12.180).
        let f = field_of("w = class { [k] = v; };");
        assert!(f.computed);
        assert!(matches!(&f.key, PropertyKey::Expression(e)
            if matches!(&**e, Expression::Identifier(id) if id.name == "k")));
        assert!(matches!(&f.value, Some(Expression::Identifier(id)) if id.name == "v"));
    }

    #[test]
    fn object_computed_key() {
        // `{ [k]: v }` — an object computed key also bridges to
        // `PropertyKey::Expression` with `computed: true`.
        let p = bridge_ok("x = { [k]: v };");
        let Expression::AssignmentExpression(a) = first_expr(&p) else {
            panic!("expected assignment")
        };
        let Expression::ObjectExpression(o) = &*a.right else { panic!("expected object") };
        let ObjectMember::Property(prop) = &o.properties[0] else { panic!("expected property") };
        assert!(prop.computed);
        assert!(matches!(&prop.key, PropertyKey::Expression(e)
            if matches!(&**e, Expression::Identifier(id) if id.name == "k")));
    }

    #[test]
    fn class_private_field_with_initializer() {
        // `#x = 1;` — a private field carries a bare `PRIVATE_NAME` token instead
        // of a `property_name` node; the bridge lowers it to
        // `PropertyKey::PrivateName` with the leading `#` stripped (CLOC12.177 PR2).
        let f = field_of("w = class { #x = 1; };");
        assert!(!f.is_static);
        assert!(!f.computed);
        assert!(
            matches!(&f.key, PropertyKey::PrivateName(p) if p.name == "x"),
            "expected a private-name key `#x` (stored bare), got {:?}",
            f.key
        );
        match &f.value {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 1.0),
            other => panic!("expected a numeric initializer, got {other:?}"),
        }
    }

    #[test]
    fn class_bare_private_field() {
        // `#x;` — a bare private field, no initializer.
        let f = field_of("z = class { #x; };");
        assert!(matches!(&f.key, PropertyKey::PrivateName(p) if p.name == "x"));
        assert!(f.value.is_none());
    }

    #[test]
    fn class_static_private_field() {
        // `static #x = 1;` — the `static` token precedes the `#x` PRIVATE_NAME
        // token; `is_static` is set and the key is still a private name.
        let f = field_of("w = class { static #x = 1; };");
        assert!(f.is_static);
        assert!(matches!(&f.key, PropertyKey::PrivateName(p) if p.name == "x"));
        match &f.value {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 1.0),
            other => panic!("expected a numeric initializer, got {other:?}"),
        }
    }

    #[test]
    fn class_private_and_public_field_interleave() {
        // A private field and a public field coexist in source order, each with
        // the right key kind.
        let c = class_of("w = class { #x = 1; y = 2; };");
        assert_eq!(c.body.len(), 2);
        assert!(matches!(&c.body[0], ClassMember::Field(f)
            if matches!(&f.key, PropertyKey::PrivateName(p) if p.name == "x")));
        assert!(matches!(&c.body[1], ClassMember::Field(f)
            if matches!(&f.key, PropertyKey::Identifier(id) if id.name == "y")));
    }

    /// Pull the sole `ClassMember::Method` out of `x = class { … };`.
    fn method_of(src: &str) -> MethodDefinition {
        let c = class_of(src);
        assert_eq!(c.body.len(), 1, "expected exactly one member");
        match &c.body[0] {
            ClassMember::Method(m) => m.clone(),
            other => panic!("expected a method member, got {other:?}"),
        }
    }

    #[test]
    fn class_private_method() {
        // `#m(){}` — a private method is a separate `private_method_definition`
        // grammar node; the bridge lowers it to a `ClassMember::Method` whose key
        // is a `PropertyKey::PrivateName` (CLOC12.178 PR1).
        let m = method_of("w = class { #m(){} };");
        assert!(!m.is_static);
        assert!(!m.computed);
        assert!(matches!(m.kind, MethodKind::Method));
        assert!(
            matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "m"),
            "expected a private-name key `#m` (stored bare), got {:?}",
            m.key
        );
        assert!(m.value.params.is_empty());
        assert!(m.value.body.body.is_empty());
    }

    #[test]
    fn class_private_method_with_params_and_body() {
        // `#add(a,b){ return a+b; }` — params and a body are collected.
        let m = method_of("w = class { #add(a, b){ return a + b; } };");
        assert!(matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "add"));
        assert_eq!(m.value.params.len(), 2);
        assert_eq!(m.value.body.body.len(), 1);
    }

    #[test]
    fn class_static_private_method() {
        // `static #m(){}` — the `static` keyword lives INSIDE the
        // `private_method_definition` node (unlike a public method's `static`,
        // which sits on the `class_element`); `is_static` is read from there.
        let m = method_of("w = class { static #m(){} };");
        assert!(m.is_static);
        assert!(matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "m"));
    }

    #[test]
    fn class_private_method_and_field_interleave() {
        // A private method and a private field coexist in source order.
        let c = class_of("w = class { #x = 1; #m(){} };");
        assert_eq!(c.body.len(), 2);
        assert!(matches!(&c.body[0], ClassMember::Field(f)
            if matches!(&f.key, PropertyKey::PrivateName(p) if p.name == "x")));
        assert!(matches!(&c.body[1], ClassMember::Method(m)
            if matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "m")));
    }

    #[test]
    fn class_private_getter() {
        // `get #x(){}` — a private getter lowers to a `MethodKind::Get` method
        // with a private-name key (CLOC12.179).
        let m = method_of("w = class { get #x(){} };");
        assert!(matches!(m.kind, MethodKind::Get));
        assert!(matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "x"));
        assert!(m.value.params.is_empty());
    }

    #[test]
    fn class_private_setter() {
        // `set #x(v){}` — a private setter lowers to a `MethodKind::Set` method
        // with a private-name key and its single parameter.
        let m = method_of("w = class { set #x(v){} };");
        assert!(matches!(m.kind, MethodKind::Set));
        assert!(matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "x"));
        assert_eq!(m.value.params.len(), 1);
    }

    #[test]
    fn class_static_private_getter() {
        // `static get #x(){}` — the `static` and `get` keywords both precede the
        // private key inside the node.
        let m = method_of("w = class { static get #x(){} };");
        assert!(m.is_static);
        assert!(matches!(m.kind, MethodKind::Get));
        assert!(matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "x"));
    }

    #[test]
    fn class_private_generator() {
        // `*#m(){}` — a private *generator* bridges (CLOC12.182): a plain
        // `MethodKind::Method` with a private-name key whose value's `generator`
        // flag is set so the emitter reprints the `*`. `yield` in the body is a
        // modelled `YieldExpression`.
        let m = method_of("w = class { *#m(){ yield 1 } };");
        assert!(matches!(m.kind, MethodKind::Method));
        assert!(m.value.generator);
        assert!(!m.value.is_async);
        assert!(matches!(&m.key, PropertyKey::PrivateName(p) if p.name == "m"));
    }

    #[test]
    fn class_static_private_generator() {
        // `static *#m(){}` — the `static` and `*` markers both precede the
        // private key inside the node; the generator flag is still set.
        let m = method_of("w = class { static *#m(){} };");
        assert!(m.is_static);
        assert!(m.value.generator);
        assert!(matches!(m.kind, MethodKind::Method));
    }

    #[test]
    fn class_private_async_method_declines() {
        // `async #m(){}` — a private *async* method carries `await` semantics not
        // yet modelled (grammar-blocked); it still DECLINES (safe WHITESPACE_ONLY),
        // never a mis-emit.
        assert!(matches!(
            bridge("w = class { async #m(){} };"),
            Err(BridgeError::UnsupportedSyntax { .. })
        ));
    }

    // -----------------------------------------------------------------
    // ClassMember::StaticBlock bridging (CLOC12.176 PR2)
    // -----------------------------------------------------------------

    /// Pull the sole `ClassMember::StaticBlock` body out of
    /// `x = class { static { … } };`.
    fn static_block_of(src: &str) -> BlockStatement {
        let c = class_of(src);
        assert_eq!(c.body.len(), 1, "expected exactly one member");
        match &c.body[0] {
            ClassMember::StaticBlock(b) => b.clone(),
            other => panic!("expected a static-block member, got {other:?}"),
        }
    }

    #[test]
    fn class_static_block_empty() {
        // `static {}` — an empty static block maps to an empty body. The block's
        // OWN leading `static` token (inside `static_block`, not on the
        // `class_element`) needs no handling.
        let b = static_block_of("y = class { static {} };");
        assert!(b.body.is_empty());
    }

    #[test]
    fn class_static_block_with_statement() {
        // `static { x = 1; }` — one expression statement in the body.
        use coding_adventures_javascript_ast::statement::TaggedStatement;
        let b = static_block_of("y = class { static { x = 1; } };");
        assert_eq!(b.body.len(), 1);
        assert!(matches!(
            &b.body[0],
            Statement::Tagged(TaggedStatement::ExpressionStatement(_))
        ));
    }

    #[test]
    fn class_static_block_with_declaration() {
        // `static { let z = 2; }` — a lexical declaration in a static block maps
        // to `Statement::Declaration` via the shared statement converter, proving
        // the full statement surface (not just expressions) is reachable.
        let b = static_block_of("y = class { static { let z = 2; } };");
        assert_eq!(b.body.len(), 1);
        assert!(matches!(
            &b.body[0],
            Statement::Declaration(Declaration::VariableDeclaration(_))
        ));
    }

    #[test]
    fn class_static_block_multiple_statements() {
        // `static { x = 1; y = 2; }` — statement order is preserved.
        use coding_adventures_javascript_ast::statement::TaggedStatement;
        let b = static_block_of("y = class { static { x = 1; y = 2; } };");
        assert_eq!(b.body.len(), 2);
        assert!(b
            .body
            .iter()
            .all(|s| matches!(s, Statement::Tagged(TaggedStatement::ExpressionStatement(_)))));
    }

    #[test]
    fn class_static_block_and_field_interleave() {
        // A static block and a field coexist in one body, in source order.
        let c = class_of("w = class { x = 1; static { y = 2; } m(){} };");
        assert_eq!(c.body.len(), 3);
        assert!(matches!(&c.body[0], ClassMember::Field(f)
            if matches!(&f.key, PropertyKey::Identifier(id) if id.name == "x")));
        assert!(matches!(&c.body[1], ClassMember::StaticBlock(b) if b.body.len() == 1));
        assert!(matches!(&c.body[2], ClassMember::Method(m)
            if matches!(&m.key, PropertyKey::Identifier(id) if id.name == "m")));
    }

    #[test]
    fn class_field_declaration_form() {
        // The field surface works in *declaration* position too (the body
        // conversion is shared between class expression and declaration).
        let c = class_decl_of("class C { x = 1; }");
        assert_eq!(c.body.len(), 1);
        assert!(matches!(&c.body[0], ClassMember::Field(f)
            if matches!(&f.key, PropertyKey::Identifier(id) if id.name == "x")));
    }

    // -----------------------------------------------------------------
    // ClassDeclaration bridging (CLOC12.174 PR2)
    // -----------------------------------------------------------------

    /// Pull the `ClassDeclaration` out of a top-level `class … { … }` statement.
    fn class_decl_of(src: &str) -> ClassDeclaration {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Declaration(Declaration::ClassDeclaration(c)) => c.clone(),
            other => panic!("expected a ClassDeclaration, got {other:?}"),
        }
    }

    #[test]
    fn class_decl_empty() {
        let c = class_decl_of("class C {}");
        assert_eq!(c.id.name, "C");
        assert!(c.super_class.is_none());
        assert!(c.body.is_empty());
    }

    #[test]
    fn class_decl_extends_identifier() {
        let c = class_decl_of("class C extends B {}");
        assert_eq!(c.id.name, "C");
        match c.super_class.as_deref() {
            Some(Expression::Identifier(id)) => assert_eq!(id.name, "B"),
            other => panic!("expected Identifier super-class, got {other:?}"),
        }
    }

    #[test]
    fn class_decl_extends_member() {
        // `extends ns.B` — heritage is a member expression (a node operand).
        let c = class_decl_of("class C extends ns.B {}");
        assert!(matches!(
            c.super_class.as_deref(),
            Some(Expression::MemberExpression(_))
        ));
    }

    #[test]
    fn class_decl_single_method() {
        let c = class_decl_of("class C { m(){} }");
        assert_eq!(c.body.len(), 1);
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(matches!(m.kind, MethodKind::Method));
        assert!(!m.is_static);
        match &m.key {
            PropertyKey::Identifier(id) => assert_eq!(id.name, "m"),
            other => panic!("expected identifier key, got {other:?}"),
        }
    }

    #[test]
    fn class_decl_static_method() {
        let c = class_decl_of("class C { static m(){} }");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(m.is_static);
    }

    #[test]
    fn class_decl_constructor() {
        let c = class_decl_of("class C { constructor(){} }");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(matches!(m.kind, MethodKind::Constructor));
    }

    #[test]
    fn class_decl_getter_setter() {
        let g = class_decl_of("class C { get x(){} }");
        let ClassMember::Method(gm) = &g.body[0] else { panic!("expected a method member") };
        assert!(matches!(gm.kind, MethodKind::Get));
        let s = class_decl_of("class C { set x(v){} }");
        let ClassMember::Method(sm) = &s.body[0] else { panic!("expected a method member") };
        assert!(matches!(sm.kind, MethodKind::Set));
    }

    #[test]
    fn class_decl_full_shape() {
        // `class C extends B { m(){} }` — name + heritage + one member together.
        let c = class_decl_of("class C extends B { m(){} }");
        assert_eq!(c.id.name, "C");
        assert!(c.super_class.is_some());
        assert_eq!(c.body.len(), 1);
    }

    #[test]
    fn class_decl_generator_method_bridges() {
        // `*m(){}` in a class *declaration* bridges (CLOC12.181): the value's
        // `generator` flag is set so the emitter reprints the `*`.
        let c = class_decl_of("class C { *m(){} }");
        let ClassMember::Method(m) = &c.body[0] else { panic!("expected a method member") };
        assert!(m.value.generator);
        assert_eq!(m.kind, MethodKind::Method);
    }

    #[test]
    fn class_decl_async_method_declines() {
        assert!(matches!(
            bridge("class C { async am(){} }"),
            Err(BridgeError::UnsupportedSyntax { .. })
        ));
    }

    #[test]
    fn regex_simple_with_flags() {
        // `/ab+c/gi` — a plain pattern with two flags.
        let r = regex_of("x = /ab+c/gi;");
        assert_eq!(r.pattern, "ab+c");
        assert_eq!(r.flags, "gi");
    }

    #[test]
    fn regex_escaped_slash_is_not_the_delimiter() {
        // `/a\/b/` — the `\/` is an escaped slash INSIDE the pattern; the
        // closing delimiter is the final `/`, so pattern = `a\/b`, no flags.
        let r = regex_of("x = /a\\/b/;");
        assert_eq!(r.pattern, "a\\/b");
        assert_eq!(r.flags, "");
    }

    #[test]
    fn regex_single_char_no_flags() {
        // `/a/` — the minimal case.
        let r = regex_of("x = /a/;");
        assert_eq!(r.pattern, "a");
        assert_eq!(r.flags, "");
    }

    #[test]
    fn regex_char_class_bridges() {
        // `/[abc]/` — a character class round-trips through the bridge. (The
        // trickier `/[/]/`, where a `/` lives inside the class, is not yet
        // tokenised by the lexer — it stops the literal at the inner `/`. Our
        // splitter already handles that shape correctly; see
        // `split_regex_literal_cases`. Enabling it end-to-end is a lexer gap
        // tracked separately, not a bridge concern.)
        let r = regex_of("x = /[abc]/;");
        assert_eq!(r.pattern, "[abc]");
        assert_eq!(r.flags, "");
    }

    // Unit tests for the low-level splitter, independent of the grammar/lexer.
    #[test]
    fn split_regex_literal_cases() {
        assert_eq!(
            split_regex_literal("/ab+c/gi"),
            Some(("ab+c".to_string(), "gi".to_string()))
        );
        assert_eq!(
            split_regex_literal("/a\\/b/"),
            Some(("a\\/b".to_string(), "".to_string()))
        );
        assert_eq!(split_regex_literal("/a/"), Some(("a".to_string(), "".to_string())));
        // Char class: the inner `/` is literal, closing `/` is the last one.
        assert_eq!(split_regex_literal("/[/]/"), Some(("[/]".to_string(), "".to_string())));
        // Escaped opening bracket means we are NOT in a class, so the next `/`
        // closes: `/\[/` → pattern `\[`, flags empty.
        assert_eq!(split_regex_literal("/\\[/"), Some(("\\[".to_string(), "".to_string())));
        // Malformed: no opening slash, and no closing slash.
        assert_eq!(split_regex_literal("abc"), None);
        assert_eq!(split_regex_literal("/abc"), None);
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

    /// A STRING literal whose *content* is a keyword must bridge to a
    /// `StringLiteral`, never to the keyword primary. `convert_primary_token`
    /// used to match `t.value` before the type discriminant, so `"true"` became
    /// `BooleanLiteral(true)` and `"this"` became `ThisExpression` — a hard
    /// miscompile: `f("true")` would call `f` with the boolean, `f("this")` with
    /// the `this` value. The value match is now gated on the token *type*.
    #[test]
    fn string_literal_with_keyword_content_stays_a_string() {
        use coding_adventures_javascript_ast::statement::TaggedStatement;
        for (src, want) in [
            ("\"true\";", "true"),
            ("\"false\";", "false"),
            ("\"null\";", "null"),
            ("\"undefined\";", "undefined"),
            ("\"this\";", "this"),
        ] {
            let p = bridge_ok(src);
            match &p.body[0] {
                ProgramItem::Statement(Statement::Tagged(
                    TaggedStatement::ExpressionStatement(es),
                )) => match &es.expression {
                    Expression::StringLiteral(s) => assert_eq!(s.value, want, "for {src}"),
                    other => panic!("expected StringLiteral({want:?}) for {src}, got {other:?}"),
                },
                other => panic!("expected an ExpressionStatement for {src}, got {other:?}"),
            }
        }
    }

    #[test]
    fn legacy_octal_string_escapes_decode() {
        use coding_adventures_javascript_ast::statement::TaggedStatement;
        // `\NNN` (ECMAScript Annex B.1.2) decodes to the code unit `0..=255`.
        // A leading digit 0-3 admits up to three octal digits; 4-7 admits two.
        for (src, want) in [
            (r#""\101";"#, "A"),        // 0o101 = 65 = 'A'
            (r#""\0";"#, "\u{0}"),      // NUL — the lone-`\0` case still works
            (r#""\012";"#, "\n"),       // 0o12 = 10 = LF
            (r#""\7";"#, "\u{7}"),      // single octal digit
            (r#""\77";"#, "?"),         // 0o77 = 63 = '?'
            (r#""\377";"#, "\u{ff}"),   // the max, 0o377 = 255
            (r#""\40";"#, " "),         // 0o40 = 32 = space
            (r#""a\101b";"#, "aAb"),    // mid-string
            (r#""\1010";"#, "A0"),      // 0o101='A' then a literal '0' (3-digit cap)
            (r#""\401";"#, "\u{101}"),   // three digits even w/ leading 4-7 (Closure rule)
            (r#""\777";"#, "\u{1ff}"),   // the max: 0o777 = 511
        ] {
            let p = bridge_ok(src);
            match &p.body[0] {
                ProgramItem::Statement(Statement::Tagged(
                    TaggedStatement::ExpressionStatement(es),
                )) => match &es.expression {
                    Expression::StringLiteral(s) => assert_eq!(s.value, want, "for {src}"),
                    other => panic!("expected StringLiteral for {src}, got {other:?}"),
                },
                other => panic!("expected an ExpressionStatement for {src}, got {other:?}"),
            }
        }
    }

    /// The genuine keyword primaries must still bridge to their literal nodes —
    /// the type gate keeps `Name`/`Keyword` tokens on the value-match path.
    #[test]
    fn bare_keyword_primaries_still_bridge() {
        use coding_adventures_javascript_ast::statement::TaggedStatement;
        let expr_of = |src: &str| -> Expression {
            let p = bridge_ok(src);
            match &p.body[0] {
                ProgramItem::Statement(Statement::Tagged(
                    TaggedStatement::ExpressionStatement(es),
                )) => es.expression.clone(),
                other => panic!("expected an ExpressionStatement for {src}, got {other:?}"),
            }
        };
        assert!(matches!(expr_of("this;"), Expression::ThisExpression(_)));
        assert!(matches!(expr_of("null;"), Expression::NullLiteral(_)));
        assert!(matches!(expr_of("undefined;"), Expression::UndefinedLiteral(_)));
        assert!(matches!(expr_of("true;"), Expression::BooleanLiteral(b) if b.value));
        assert!(matches!(expr_of("false;"), Expression::BooleanLiteral(b) if !b.value));
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

    /// `super.x` — `super` (gap-167, CLOC12.166 PR2) bridges as the object of a
    /// member access rather than being declined as `UnsupportedSyntax`. Unlike
    /// `this`, `super` is a bare token base folded through the member suffix
    /// chain, so a member access is the canonical shape (bare `super` is not
    /// valid JS).
    #[test]
    fn super_member_object() {
        let p = bridge_ok("super.x;");
        match only_expr(&p) {
            Expression::MemberExpression(m) => {
                assert!(matches!(&*m.object, Expression::Super(_)));
            }
            other => panic!("expected MemberExpression; got {:?}", other),
        }
    }

    /// `super[k]` — the computed-member form also folds onto the `Super` base.
    #[test]
    fn super_computed_member_object() {
        let p = bridge_ok("super[k];");
        match only_expr(&p) {
            Expression::MemberExpression(m) => {
                assert!(m.computed);
                assert!(matches!(&*m.object, Expression::Super(_)));
            }
            other => panic!("expected MemberExpression; got {:?}", other),
        }
    }

    /// `super.m(1 + 2)` — a method call off `super`: the callee is a
    /// member access whose object is `Super`, and the argument survives so the
    /// downstream constant-fold can reduce it.
    #[test]
    fn super_method_call() {
        let p = bridge_ok("super.m(1 + 2);");
        match only_expr(&p) {
            Expression::CallExpression(c) => match &*c.callee {
                Expression::MemberExpression(m) => {
                    assert!(matches!(&*m.object, Expression::Super(_)));
                    assert_eq!(c.arguments.len(), 1);
                }
                other => panic!("expected MemberExpression callee; got {:?}", other),
            },
            other => panic!("expected CallExpression; got {:?}", other),
        }
    }

    /// `new.target` — the meta-property (gap-168, CLOC12.167 PR2). The grammar
    /// emits it as three bare tokens (`new`, `.`, `target`) in a
    /// `member_expression` with no Node child, so the bridge must intercept it
    /// as an atomic `NewTarget` leaf (the `.` is spelling, not a member
    /// access). A bare `new.target;` parses standalone here.
    #[test]
    fn new_target_meta_property() {
        let p = bridge_ok("new.target;");
        assert!(
            matches!(only_expr(&p), Expression::NewTarget(_)),
            "expected NewTarget; got {:?}",
            only_expr(&p)
        );
    }

    /// `new.target` inside a function body (its canonical legal position) also
    /// bridges cleanly — `bridge_ok` panics if the bridge declines, so a
    /// successful parse+bridge here proves the meta-property is handled in a
    /// nested (`return`) position too, not only at statement top level.
    #[test]
    fn new_target_in_function_return() {
        let _ = bridge_ok("function f(){return new.target;}");
    }

    /// `new.target` used as a member object (`new.target.x`) still bridges: the
    /// meta-property becomes the object of the outer member access. This pins
    /// that the leaf slots into the suffix-fold like any other base primary.
    #[test]
    fn new_target_as_member_object() {
        let p = bridge_ok("new.target.constructor;");
        match only_expr(&p) {
            Expression::MemberExpression(m) => assert!(
                matches!(&*m.object, Expression::NewTarget(_)),
                "expected NewTarget object; got {:?}",
                m.object
            ),
            other => panic!("expected MemberExpression; got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // import.meta (CLOC12.168 PR2, gap-169)
    // -----------------------------------------------------------------------

    /// `import.meta` — the module meta-property, the sibling of `new.target`.
    /// The grammar emits a dedicated `import_meta` leaf (`[Token("import"),
    /// Token("."), Token("meta")]`, no Node child); the bridge intercepts it as
    /// an atomic `ImportMeta` leaf (the `.meta` is spelling, not member access).
    /// A bare `import.meta;` parses standalone here. Previously the rule fell
    /// through to the internal-error arm, dropping the file to WHITESPACE_ONLY.
    #[test]
    fn import_meta_meta_property() {
        let p = bridge_ok("import.meta;");
        assert!(
            matches!(only_expr(&p), Expression::ImportMeta(_)),
            "expected ImportMeta; got {:?}",
            only_expr(&p)
        );
    }

    /// `import.meta.url` — the canonical use, as the object of a member access.
    /// The meta-property becomes the object of the outer member, pinning that
    /// the leaf slots into the suffix-fold like any other base primary.
    #[test]
    fn import_meta_as_member_object() {
        let p = bridge_ok("import.meta.url;");
        match only_expr(&p) {
            Expression::MemberExpression(m) => assert!(
                matches!(&*m.object, Expression::ImportMeta(_)),
                "expected ImportMeta object; got {:?}",
                m.object
            ),
            other => panic!("expected MemberExpression; got {:?}", other),
        }
    }

    /// `f(import.meta)` — the meta-property as a call argument bridges cleanly
    /// (a plain primary operand), proving it is handled in nested position too.
    #[test]
    fn import_meta_as_call_argument() {
        let _ = bridge_ok("f(import.meta);");
    }

    // -----------------------------------------------------------------------
    // import(x) — dynamic import (CLOC12.169 PR2, gap-170)
    // -----------------------------------------------------------------------

    /// `import("m")` — the canonical dynamic import of a string module specifier.
    /// The grammar emits `dynamic_import` (`[Token("import"), Token("("),
    /// Node(source), Token(")")]`); the bridge lowers it to the compound
    /// `ImportExpression` whose `source` is the converted specifier — here a
    /// `StringLiteral`. Previously the rule fell through to the internal-error
    /// arm, dropping the file to WHITESPACE_ONLY.
    #[test]
    fn dynamic_import_string_specifier() {
        let p = bridge_ok("import(\"m\");");
        match only_expr(&p) {
            Expression::ImportExpression(ie) => match &*ie.source {
                Expression::StringLiteral(s) => assert_eq!(s.value, "m"),
                other => panic!("expected StringLiteral source; got {:?}", other),
            },
            other => panic!("expected ImportExpression; got {:?}", other),
        }
    }

    /// `import(x)` — a dynamic import of a *variable* specifier (computed at
    /// runtime). Pins that the `source` operand is recursively converted, so a
    /// non-literal specifier lowers to an `Identifier` rather than being dropped.
    #[test]
    fn dynamic_import_identifier_specifier() {
        let p = bridge_ok("import(x);");
        match only_expr(&p) {
            Expression::ImportExpression(ie) => assert!(
                matches!(&*ie.source, Expression::Identifier(_)),
                "expected Identifier source; got {:?}",
                ie.source
            ),
            other => panic!("expected ImportExpression; got {:?}", other),
        }
    }

    /// `f(import("m"))` — the dynamic import as a call argument bridges cleanly
    /// (a compound operand in nested position), proving it slots into the
    /// argument list like any other expression.
    #[test]
    fn dynamic_import_as_call_argument() {
        let _ = bridge_ok("f(import(\"m\"));");
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

    /// Pull the C-style `ForStatement` out of a program whose first statement is
    /// `for (…;…;…) …`.
    fn bridge_for(src: &str) -> ForStatement {
        let p = bridge_ok(src);
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::ForStatement(f),
            )) => f.clone(),
            other => panic!("expected a ForStatement, got {other:?}"),
        }
    }

    #[test]
    fn for_lexical_init_bridges() {
        // CLOC12.186: a `let` / `const` init in a C-style `for` header. Before
        // this the init's `binding_list` node fell through to `convert_expression`
        // and raised an InternalError ("unknown expression rule 'binding_list'"),
        // declining the whole file to WHITESPACE_ONLY.
        for (src, want) in [
            ("for (let i = 0; i < 3; i++) f();", VarKind::Let),
            ("for (const j = 1; ; ) f();", VarKind::Const),
        ] {
            let f = bridge_for(src);
            match &f.init {
                Some(ForInit::VariableDeclaration(v)) => {
                    assert_eq!(v.kind, want, "for {src}");
                    assert_eq!(v.declarations.len(), 1, "for {src}");
                    assert!(v.declarations[0].init.is_some(), "init has a value for {src}");
                }
                other => panic!("expected a {want:?} declaration init for {src}, got {other:?}"),
            }
        }
    }

    #[test]
    fn for_lexical_init_multi_binding_bridges() {
        // `for (let a = 1, b = 2; …)` — two declarators in the lexical init.
        let f = bridge_for("for (let a = 1, b = 2; a < b; a++) f();");
        match &f.init {
            Some(ForInit::VariableDeclaration(v)) => {
                assert_eq!(v.kind, VarKind::Let);
                assert_eq!(v.declarations.len(), 2);
            }
            other => panic!("expected a let declaration init, got {other:?}"),
        }
    }

    #[test]
    fn for_var_init_still_bridges() {
        // A `var` init (the pre-existing path) still works.
        let f = bridge_for("for (var v = 0; v < 3; v++) f();");
        match &f.init {
            Some(ForInit::VariableDeclaration(v)) => assert_eq!(v.kind, VarKind::Var),
            other => panic!("expected a var declaration init, got {other:?}"),
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
        let FunctionParam::Identifier(p) = &a.params[0] else {
            panic!("expected a plain Identifier param, got {:?}", a.params[0]);
        };
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
    fn arrow_paren_object_concise_body_bridges() {
        // `() => ({a:1})` — a PARENTHESISED object-literal expression body. It is
        // now distinguishable from the bare block `() => {}` by the concise_body's
        // leftmost token (`(` vs `{`), so it bridges (CLOC12.185) instead of
        // declining. (The empty-block `() => {}` became a block at CLOC12.184.)
        assert!(
            grammar_to_program(
                &crate::parse_javascript("var f=()=>({a:1});", "es2025").expect("parse"),
                DEFAULT_ES_VERSION,
            )
            .is_ok(),
            "parenthesised object-body arrow should bridge"
        );
    }

    #[test]
    fn async_arrow_single_param_bridges() {
        // `async x => x` — CLOC12.192. Async arrows parse under
        // `async_arrow_function` (the plain arrow shape plus a leading `async`
        // literal) and now bridge to an `ArrowFunctionExpression` with `is_async`
        // set, instead of declining to WHITESPACE_ONLY.
        let f = arrow_of("x = async x => x;");
        assert!(f.is_async, "async arrow must set is_async");
        assert_eq!(f.params.len(), 1);
    }

    #[test]
    fn async_arrow_paren_params_bridges() {
        // `async (a, b) => a` — parenthesised params carry the same async flag.
        let f = arrow_of("x = async (a, b) => a;");
        assert!(f.is_async);
        assert_eq!(f.params.len(), 2);
    }

    #[test]
    fn plain_arrow_is_not_async() {
        // Regression: the plain-arrow dispatch still bridges with is_async=false
        // after the shared converter gained the `is_async` parameter.
        let f = arrow_of("x = y => y;");
        assert!(!f.is_async, "plain arrow must not be async");
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
    fn with_statement_bridge() {
        // CLOC12.187 PR2b: `with (o) { … }` now bridges to a WithStatement
        // instead of declining the whole file to WHITESPACE_ONLY. The renaming
        // passes decline to rename in its presence (the PR2a gate), so bridging
        // it is sound. Structurally it mirrors `while_statement`:
        // object = the injected expression, body = the statement.
        let p = bridge_ok("with (o) { foo(); }");
        match &p.body[0] {
            ProgramItem::Statement(Statement::Tagged(
                coding_adventures_javascript_ast::statement::TaggedStatement::WithStatement(w),
            )) => {
                assert!(
                    matches!(&w.object, Expression::Identifier(id) if id.name == "o"),
                    "expected the `with` object to be the identifier `o`, got {:?}",
                    w.object
                );
                assert!(
                    matches!(
                        &*w.body,
                        Statement::Tagged(
                            coding_adventures_javascript_ast::statement::TaggedStatement::BlockStatement(_)
                        )
                    ),
                    "expected the `with` body to be a block statement, got {:?}",
                    w.body
                );
            }
            other => panic!("expected a WithStatement, got {other:?}"),
        }
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

    // -----------------------------------------------------------------------
    // Optional chaining — `a?.b` / `a?.[k]` / `a?.()`  (CLOC12.171 PR2)
    // -----------------------------------------------------------------------

    /// `a?.b` bridges to `ChainExpression( OptionalMemberExpression{ a, b } )`.
    #[test]
    fn optional_member_dot_bridges() {
        let chain = match first_expr(&bridge_ok("a?.b;")) {
            Expression::ChainExpression(c) => c.clone(),
            other => panic!("expected ChainExpression, got {other:?}"),
        };
        match &*chain.expression {
            Expression::OptionalMemberExpression(m) => {
                assert!(!m.computed);
                assert!(matches!(&*m.object, Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(&*m.property, Expression::Identifier(i) if i.name == "b"));
            }
            other => panic!("expected OptionalMemberExpression, got {other:?}"),
        }
    }

    /// `a?.[k]` bridges to a computed `OptionalMemberExpression`.
    #[test]
    fn optional_member_computed_bridges() {
        let chain = match first_expr(&bridge_ok("a?.[k];")) {
            Expression::ChainExpression(c) => c.clone(),
            other => panic!("expected ChainExpression, got {other:?}"),
        };
        match &*chain.expression {
            Expression::OptionalMemberExpression(m) => {
                assert!(m.computed, "?.[k] is a computed access");
                assert!(matches!(&*m.object, Expression::Identifier(i) if i.name == "a"));
                assert!(matches!(&*m.property, Expression::Identifier(i) if i.name == "k"));
            }
            other => panic!("expected OptionalMemberExpression, got {other:?}"),
        }
    }

    /// `a?.()` bridges to `ChainExpression( OptionalCallExpression{ a, [] } )`.
    #[test]
    fn optional_call_bridges() {
        let chain = match first_expr(&bridge_ok("a?.();")) {
            Expression::ChainExpression(c) => c.clone(),
            other => panic!("expected ChainExpression, got {other:?}"),
        };
        match &*chain.expression {
            Expression::OptionalCallExpression(c) => {
                assert!(c.arguments.is_empty());
                assert!(matches!(&*c.callee, Expression::Identifier(i) if i.name == "a"));
            }
            other => panic!("expected OptionalCallExpression, got {other:?}"),
        }
    }

    /// `a?.b.c` — only the FIRST link is optional. The `.c` that follows is an
    /// ordinary `MemberExpression` whose object is the optional node, and the
    /// whole spine is wrapped once in a single `ChainExpression`.
    #[test]
    fn optional_then_plain_link_wraps_once() {
        let chain = match first_expr(&bridge_ok("a?.b.c;")) {
            Expression::ChainExpression(c) => c.clone(),
            other => panic!("expected ChainExpression, got {other:?}"),
        };
        match &*chain.expression {
            Expression::MemberExpression(outer) => {
                // `.c` is a PLAIN member.
                assert!(!outer.computed);
                assert!(matches!(&*outer.property, Expression::Identifier(i) if i.name == "c"));
                // Its object is the OPTIONAL `a?.b`.
                match &*outer.object {
                    Expression::OptionalMemberExpression(inner) => {
                        assert!(matches!(&*inner.object, Expression::Identifier(i) if i.name == "a"));
                        assert!(matches!(&*inner.property, Expression::Identifier(i) if i.name == "b"));
                    }
                    other => panic!("expected inner OptionalMemberExpression, got {other:?}"),
                }
            }
            other => panic!("expected an outer plain MemberExpression, got {other:?}"),
        }
    }

    /// A chain with NO optional link is NOT wrapped in a `ChainExpression` —
    /// `a.b` stays a bare `MemberExpression`, exactly as before.
    #[test]
    fn plain_chain_is_not_wrapped() {
        assert!(
            matches!(first_expr(&bridge_ok("a.b;")), Expression::MemberExpression(_)),
            "a plain member access must not be wrapped in a ChainExpression",
        );
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
            Expression::ObjectExpression(o) => match &o.properties[0] {
                ObjectMember::Property(p) => p.key.clone(),
                ObjectMember::Spread(_) => {
                    unreachable!("first_object_key fixtures build no object spreads")
                }
            },
            other => panic!("expected ObjectExpression, got {other:?}"),
        }
    }

    // ---- object spread `{...o}` (CLOC12.170 PR2, closes gap-SpreadProperty) --

    /// `({...o})` bridges to an `ObjectExpression` whose sole member is an
    /// `ObjectMember::Spread` wrapping the identifier `o` (no longer declined to
    /// `SpreadProperty` / WHITESPACE_ONLY).
    #[test]
    fn object_spread_sole_member() {
        match first_expr(&bridge_ok("({...o});")) {
            Expression::ObjectExpression(obj) => {
                assert_eq!(obj.properties.len(), 1, "one (spread) member");
                match &obj.properties[0] {
                    ObjectMember::Spread(s) => {
                        assert!(matches!(&*s.argument, Expression::Identifier(i) if i.name == "o"));
                    }
                    other => panic!("expected Spread member, got {other:?}"),
                }
            }
            other => panic!("expected ObjectExpression, got {other:?}"),
        }
    }

    /// `({a: 1, ...o})` preserves member order — a plain `Property` then a
    /// `Spread` (interleaving is observable: the spread may override `a`).
    #[test]
    fn object_spread_after_property_preserves_order() {
        match first_expr(&bridge_ok("({a: 1, ...o});")) {
            Expression::ObjectExpression(obj) => {
                assert_eq!(obj.properties.len(), 2, "two members in order");
                assert!(matches!(&obj.properties[0], ObjectMember::Property(p)
                    if matches!(&p.key, PropertyKey::Identifier(i) if i.name == "a")));
                assert!(matches!(&obj.properties[1], ObjectMember::Spread(s)
                    if matches!(&*s.argument, Expression::Identifier(i) if i.name == "o")));
            }
            other => panic!("expected ObjectExpression, got {other:?}"),
        }
    }

    /// `f({...o})` — an object spread nested inside a call argument bridges
    /// cleanly (the whole file no longer drops to WHITESPACE_ONLY).
    #[test]
    fn object_spread_in_call_argument() {
        match first_expr(&bridge_ok("f({...o});")) {
            Expression::CallExpression(c) => match &c.arguments[0] {
                Expression::ObjectExpression(obj) => {
                    assert!(matches!(&obj.properties[0], ObjectMember::Spread(s)
                        if matches!(&*s.argument, Expression::Identifier(i) if i.name == "o")));
                }
                other => panic!("expected ObjectExpression argument, got {other:?}"),
            },
            other => panic!("expected CallExpression, got {other:?}"),
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
