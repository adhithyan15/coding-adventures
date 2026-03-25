//! Engine — public entry points for SQL execution.
//!
//! Two functions:
//! - [`execute`] — parse and execute a single SQL SELECT statement.
//! - [`execute_all`] — parse and execute all SELECT statements.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use coding_adventures_sql_parser::parse_sql;

use crate::data_source::DataSource;
use crate::errors::ExecutionError;
use crate::executor::execute_select;
use crate::result::QueryResult;

/// Parse and execute a single SQL SELECT statement.
///
/// # Errors
///
/// - [`ExecutionError::ParseError`] if the SQL has syntax errors.
/// - [`ExecutionError::TableNotFound`] if a table is unknown.
/// - [`ExecutionError::ColumnNotFound`] if a column reference is invalid.
pub fn execute(sql: &str, source: &dyn DataSource) -> Result<QueryResult, ExecutionError> {
    let ast = parse_sql(sql)
        .map_err(|e| ExecutionError::ParseError(e))?;
    match find_first_select(&ast) {
        Some(stmt) => execute_select(stmt, source),
        None => Ok(QueryResult::empty()),
    }
}

/// Parse and execute all SELECT statements in a SQL string.
///
/// Non-SELECT statements are silently skipped.
///
/// # Errors
///
/// Returns the first error encountered during execution.
pub fn execute_all(sql: &str, source: &dyn DataSource) -> Result<Vec<QueryResult>, ExecutionError> {
    let ast = parse_sql(sql)
        .map_err(|e| ExecutionError::ParseError(e))?;
    let mut results = Vec::new();
    for stmt in find_all_selects(&ast) {
        results.push(execute_select(stmt, source)?);
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// AST navigation helpers
// ---------------------------------------------------------------------------

fn find_first_select(ast: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if ast.rule_name == "select_stmt" { return Some(ast); }
    for child in &ast.children {
        if let ASTNodeOrToken::Node(n) = child {
            if let Some(found) = find_first_select(n) {
                return Some(found);
            }
        }
    }
    None
}

fn find_all_selects(ast: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    let mut results = Vec::new();
    collect_selects(ast, &mut results);
    results
}

fn collect_selects<'a>(node: &'a GrammarASTNode, results: &mut Vec<&'a GrammarASTNode>) {
    if node.rule_name == "select_stmt" {
        results.push(node);
        return;
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            collect_selects(n, results);
        }
    }
}
