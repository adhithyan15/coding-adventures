// Package sqlengine provides a SELECT-only SQL execution engine.
//
// This file defines the error types used throughout the engine. Clear,
// typed errors are essential in a query engine because callers need to
// distinguish between "the table doesn't exist" (user error), "the column
// doesn't exist" (user error), and internal bugs. Using concrete types
// (rather than fmt.Errorf strings) allows callers to use errors.As() to
// inspect what went wrong.
//
// Design principle: each error carries the name that caused the failure so
// error messages point the user directly at the offending identifier.
package sqlengine

import "fmt"

// TableNotFoundError is returned when the DataSource has no schema for the
// requested table name. This is analogous to a "relation does not exist"
// error in PostgreSQL.
//
// Example:
//
//	SELECT * FROM nonexistent_table
//	→ TableNotFoundError{TableName: "nonexistent_table"}
type TableNotFoundError struct {
	TableName string
}

func (e *TableNotFoundError) Error() string {
	return fmt.Sprintf("table not found: %q", e.TableName)
}

// ColumnNotFoundError is returned when a query references a column that does
// not exist in the row context. This covers both:
//   - Columns that don't exist in the table's schema
//   - Qualified names (table.column) where the table alias is unknown
//
// Example:
//
//	SELECT nonexistent_col FROM employees
//	→ ColumnNotFoundError{ColumnName: "nonexistent_col"}
type ColumnNotFoundError struct {
	ColumnName string
}

func (e *ColumnNotFoundError) Error() string {
	return fmt.Sprintf("column not found: %q", e.ColumnName)
}

// UnsupportedStatementError is returned when the SQL string contains a
// statement type that the engine does not support. This engine only executes
// SELECT; INSERT, UPDATE, DELETE, CREATE TABLE, and DROP TABLE are rejected.
//
// This is a deliberate design choice: a read-only engine is safer to expose
// because it cannot mutate data.
type UnsupportedStatementError struct {
	StatementType string
}

func (e *UnsupportedStatementError) Error() string {
	return fmt.Sprintf("unsupported statement type: %s (only SELECT is supported)", e.StatementType)
}

// EvaluationError wraps errors that occur while evaluating an expression.
// For example, dividing by zero, or calling an unknown aggregate function.
type EvaluationError struct {
	Message string
}

func (e *EvaluationError) Error() string {
	return fmt.Sprintf("evaluation error: %s", e.Message)
}
