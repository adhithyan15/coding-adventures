// Package sqlengine is a SELECT-only SQL execution engine.
//
// # Architecture Overview
//
// The engine is a classic relational algebra pipeline. A SQL SELECT statement
// maps directly onto a sequence of relational operators:
//
//	SQL clause  │ Relational op    │ Go function
//	────────────┼──────────────────┼────────────────────────
//	FROM        │ Scan             │ scanTable
//	JOIN        │ Nested-loop join │ applyJoin
//	WHERE       │ Select (σ)       │ filterRows
//	GROUP BY    │ Partition        │ groupRows
//	HAVING      │ Select on groups │ filterGroups
//	SELECT      │ Project (π)      │ projectRows
//	DISTINCT    │ Deduplication    │ deduplicateRows
//	ORDER BY    │ Sort             │ sortRows
//	LIMIT/OFFSET│ Slice            │ applyLimit
//
// Each stage produces a new set of rows consumed by the next stage. This
// "volcano model" (or iterator model) is the foundation of most real database
// executors (MySQL, PostgreSQL, SQLite all use variants of this).
//
// # Usage
//
//	result, err := sqlengine.Execute("SELECT id, name FROM employees WHERE salary > 80000", source)
//	if err != nil { log.Fatal(err) }
//	for _, row := range result.Rows {
//	    fmt.Println(row)
//	}
//
// # NULL Handling
//
// The engine implements SQL's three-valued logic (TRUE, FALSE, NULL). A WHERE
// clause predicate that evaluates to NULL (not just false) also excludes the
// row. This matches standard SQL semantics.
//
// # Aggregates
//
// COUNT(*), COUNT(expr), SUM, AVG, MIN, MAX are supported. When any aggregate
// appears in the select list, the query is treated as an aggregate query even
// without a GROUP BY (in that case, all rows form one implicit group).
package sqlengine

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	sqlparser "github.com/adhithyan15/coding-adventures/code/packages/go/sql-parser"
)

// Execute parses and executes a single SQL SELECT statement against source.
//
// If sql contains multiple statements separated by ";", only the first
// statement is executed. Use ExecuteAll to execute all statements.
//
// Returns an error if:
//   - The SQL cannot be parsed (syntax error)
//   - The statement is not a SELECT (UnsupportedStatementError)
//   - A referenced table does not exist (TableNotFoundError)
//   - A referenced column does not exist (ColumnNotFoundError)
func Execute(sql string, source DataSource) (*QueryResult, error) {
	return StartNew[*QueryResult]("sql-execution-engine.Execute", nil,
		func(op *Operation[*QueryResult], rf *ResultFactory[*QueryResult]) *OperationResult[*QueryResult] {
			op.AddProperty("sql", sql)
			// Step 1: Parse the SQL text into an AST.
			ast, err := sqlparser.ParseSQL(sql)
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("parse error: %w", err))
			}

			// Step 2: Navigate to the first statement's select_stmt node.
			selectNode, err := findSelectNode(ast)
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Execute the select statement.
			result, err := executeSelect(selectNode, source)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, result)
		}).GetResult()
}

// ExecuteAll parses and executes all SQL statements in sql, returning one
// QueryResult per statement. Statements are separated by ";".
//
// This is useful for executing a SQL script or for testing multiple queries
// in sequence. Each statement is executed independently; they share only the
// DataSource (there is no transaction or shared state).
//
// Returns an error as soon as any statement fails.
func ExecuteAll(sql string, source DataSource) ([]*QueryResult, error) {
	return StartNew[[]*QueryResult]("sql-execution-engine.ExecuteAll", nil,
		func(op *Operation[[]*QueryResult], rf *ResultFactory[[]*QueryResult]) *OperationResult[[]*QueryResult] {
			op.AddProperty("sql", sql)
			ast, err := sqlparser.ParseSQL(sql)
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("parse error: %w", err))
			}

			var results []*QueryResult
			for _, child := range ast.Children {
				stmtNode, ok := child.(*parser.ASTNode)
				if !ok {
					continue
				}
				if stmtNode.RuleName != "statement" {
					continue
				}
				selectNode, err := findSelectInStatement(stmtNode)
				if err != nil {
					return rf.Fail(nil, err)
				}
				result, err := executeSelect(selectNode, source)
				if err != nil {
					return rf.Fail(nil, err)
				}
				results = append(results, result)
			}

			return rf.Generate(true, false, results)
		}).GetResult()
}

// findSelectNode navigates from a "program" root to the first "select_stmt"
// node. Returns UnsupportedStatementError if the first statement is not SELECT.
func findSelectNode(program *parser.ASTNode) (*parser.ASTNode, error) {
	return findSelectInStatement(program)
}

// findSelectInStatement extracts the select_stmt from a statement node.
// Returns UnsupportedStatementError for any non-SELECT statement type.
func findSelectInStatement(stmt *parser.ASTNode) (*parser.ASTNode, error) {
	kind, node := findFirstConcreteStatement(stmt)
	switch kind {
	case "select_stmt":
		return node, nil
	case "insert_stmt":
		return nil, &UnsupportedStatementError{StatementType: "INSERT"}
	case "update_stmt":
		return nil, &UnsupportedStatementError{StatementType: "UPDATE"}
	case "delete_stmt":
		return nil, &UnsupportedStatementError{StatementType: "DELETE"}
	case "create_table_stmt":
		return nil, &UnsupportedStatementError{StatementType: "CREATE TABLE"}
	case "drop_table_stmt":
		return nil, &UnsupportedStatementError{StatementType: "DROP TABLE"}
	}
	return nil, fmt.Errorf("could not find statement type in AST node %q", stmt.RuleName)
}

func findFirstConcreteStatement(node *parser.ASTNode) (string, *parser.ASTNode) {
	switch node.RuleName {
	case "select_stmt", "insert_stmt", "update_stmt", "delete_stmt", "create_table_stmt", "drop_table_stmt":
		return node.RuleName, node
	}
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if kind, found := findFirstConcreteStatement(childNode); found != nil {
			return kind, found
		}
	}
	return "", nil
}
