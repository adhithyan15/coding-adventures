// QueryResult holds the output of a completed SELECT query.
//
// A SELECT query produces a table: a list of named columns and a list of rows
// where each row is a slice of values. This mirrors what a real database
// driver returns (e.g., sql.Rows in the standard library) but in a simple,
// fully-materialized form that is easy to test.
//
// Why fully materialized? Because this is an educational implementation.
// Production engines stream rows lazily to avoid holding the entire result
// set in memory. For clarity and testability, we prefer materializing
// everything up front.
package sqlengine

import "fmt"

// QueryResult is the output of Execute() or ExecuteAll().
//
// Fields:
//   - Columns: the output column names, in select-list order.
//     For "SELECT id AS user_id, name FROM employees" this would be
//     ["user_id", "name"].
//   - Rows: each row is a slice of values aligned with Columns.
//     Values may be nil (SQL NULL), int64, float64, string, or bool.
type QueryResult struct {
	Columns []string
	Rows    [][]interface{}
}

// String returns a human-readable table representation of the result.
// This is useful for debugging and for printing results in a REPL.
//
// Example output:
//
//	id  | name  | salary
//	----+-------+--------
//	1   | Alice | 90000
//	2   | Bob   | 75000
func (r *QueryResult) String() string {
	if len(r.Columns) == 0 {
		return "(empty result)"
	}

	// Format each cell as a string first so we can compute column widths.
	cellStr := make([][]string, len(r.Rows))
	for i, row := range r.Rows {
		cellStr[i] = make([]string, len(row))
		for j, v := range row {
			if v == nil {
				cellStr[i][j] = "NULL"
			} else {
				cellStr[i][j] = fmt.Sprintf("%v", v)
			}
		}
	}

	// Compute per-column widths (at least as wide as the column header).
	widths := make([]int, len(r.Columns))
	for i, col := range r.Columns {
		widths[i] = len(col)
	}
	for _, row := range cellStr {
		for j, cell := range row {
			if len(cell) > widths[j] {
				widths[j] = len(cell)
			}
		}
	}

	// Build the header row.
	result := ""
	for i, col := range r.Columns {
		if i > 0 {
			result += " | "
		}
		result += fmt.Sprintf("%-*s", widths[i], col)
	}
	result += "\n"

	// Build the separator line.
	for i, w := range widths {
		if i > 0 {
			result += "-+-"
		}
		for j := 0; j < w; j++ {
			result += "-"
		}
	}
	result += "\n"

	// Build each data row.
	for _, row := range cellStr {
		for i, cell := range row {
			if i > 0 {
				result += " | "
			}
			result += fmt.Sprintf("%-*s", widths[i], cell)
		}
		result += "\n"
	}

	return result
}
