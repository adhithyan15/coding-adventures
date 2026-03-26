// JOIN execution for the SQL execution engine.
//
// SQL supports five join types, each with different semantics for handling
// rows that don't match the ON condition:
//
//	INNER JOIN:         Only rows where ON condition is TRUE. The most common join.
//	LEFT [OUTER] JOIN:  All left rows; right columns are NULL for non-matches.
//	RIGHT [OUTER] JOIN: All right rows; left columns are NULL for non-matches.
//	FULL [OUTER] JOIN:  All rows from both sides; unmatched sides are NULL.
//	CROSS JOIN:         Cartesian product of both sides (no ON condition).
//
// # Implementation: Nested Loop Join
//
// We use the simplest possible join algorithm: nested loop join.
// For each row in the left (outer) side, scan all rows in the right (inner) side
// and apply the ON condition. This is O(N × M) where N and M are table sizes.
//
// A production database would use hash join (O(N+M)) or merge join (O(N log N))
// for large tables. Nested loop join is correct and simple, which is why it
// appears in every database textbook as the baseline algorithm.
//
// # Row context merging
//
// When joining two tables, each row from the result combines column values
// from both sides. We use qualified names ("tableName.colName") in the row
// context to avoid ambiguity when both tables have a column named "id".
//
// Example: employees JOIN departments
//   employees row: {"id": 1, "name": "Alice", "dept_id": 1}
//   departments row: {"id": 1, "name": "Engineering"}
//
//   merged row: {
//     "employees.id": 1, "employees.name": "Alice", "employees.dept_id": 1,
//     "departments.id": 1, "departments.name": "Engineering"
//   }
//
// The column resolver in expression.go handles unqualified references by
// searching for suffix matches.
package sqlengine

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// applyJoin executes one join_clause against the current row set.
//
// Grammar:
//
//	join_clause = join_type "JOIN" table_ref "ON" expr
//	join_type   = "CROSS" | "INNER" | ( "LEFT" [ "OUTER" ] )
//	            | ( "RIGHT" [ "OUTER" ] ) | ( "FULL" [ "OUTER" ] )
//
// Parameters:
//   - leftRows: the current accumulated rows (result of FROM + previous JOINs)
//   - joinNode: the join_clause AST node
//   - source: the DataSource for scanning the right-side table
//   - leftTableName: the name/alias of the left table (used for qualifying columns)
//
// Returns the merged rows after applying the join.
func applyJoin(leftRows []map[string]interface{}, joinNode *parser.ASTNode, source DataSource) ([]map[string]interface{}, error) {
	// Step 1: Determine the join type.
	joinType := extractJoinType(joinNode)

	// Step 2: Find the right-side table_ref and ON expression.
	tableRefNode := findChild(joinNode, "table_ref")
	if tableRefNode == nil {
		return leftRows, nil
	}

	// Step 3: Scan the right-side table.
	rightTableName, rightAlias := extractTableNameAndAlias(tableRefNode)
	rightRows, err := source.Scan(rightTableName)
	if err != nil {
		return nil, err
	}

	// Qualify right rows with their table name prefix.
	qualifiedRight := qualifyRows(rightRows, rightAlias)

	// Step 4: Find the ON expression (for non-CROSS joins).
	var onExpr *parser.ASTNode
	if joinType != "CROSS" {
		onExpr = findOnExpr(joinNode)
	}

	// Step 5: Execute the appropriate join algorithm.
	switch joinType {
	case "INNER":
		return innerJoin(leftRows, qualifiedRight, onExpr), nil
	case "LEFT":
		cols, _ := source.Schema(rightTableName)
		return leftJoin(leftRows, qualifiedRight, onExpr, cols, rightAlias), nil
	case "RIGHT":
		cols, _ := source.Schema(rightTableName)
		return rightJoin(leftRows, qualifiedRight, onExpr, cols, rightAlias), nil
	case "FULL":
		cols, _ := source.Schema(rightTableName)
		return fullOuterJoin(leftRows, qualifiedRight, onExpr, cols, rightAlias), nil
	case "CROSS":
		return crossJoin(leftRows, qualifiedRight), nil
	default:
		// Unknown join type: fall back to inner join.
		return innerJoin(leftRows, qualifiedRight, onExpr), nil
	}
}

// extractJoinType reads the join_type node from a join_clause and returns
// a normalized string: "INNER", "LEFT", "RIGHT", "FULL", or "CROSS".
func extractJoinType(joinNode *parser.ASTNode) string {
	joinTypeNode := findChild(joinNode, "join_type")
	if joinTypeNode == nil {
		return "INNER"
	}

	for _, child := range joinTypeNode.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		switch strings.ToUpper(tok.Value) {
		case "INNER":
			return "INNER"
		case "LEFT":
			return "LEFT"
		case "RIGHT":
			return "RIGHT"
		case "FULL":
			return "FULL"
		case "CROSS":
			return "CROSS"
		}
	}
	return "INNER"
}

// findOnExpr locates the ON expression in a join_clause.
// The grammar is: join_type "JOIN" table_ref "ON" expr
// We find the "ON" keyword token and then take the next ASTNode child.
func findOnExpr(joinNode *parser.ASTNode) *parser.ASTNode {
	foundOn := false
	for _, child := range joinNode.Children {
		if tok, ok := child.(lexer.Token); ok {
			if strings.ToUpper(tok.Value) == "ON" {
				foundOn = true
				continue
			}
		}
		if foundOn {
			if node, ok := child.(*parser.ASTNode); ok {
				return node
			}
		}
	}
	return nil
}

// qualifyRows prefixes all column names with "tableName." to avoid ambiguity
// in joined row contexts.
//
// Example: {"id": 1, "name": "Alice"} with prefix "employees" becomes
// {"employees.id": 1, "employees.name": "Alice"}
func qualifyRows(rows []map[string]interface{}, tableName string) []map[string]interface{} {
	result := make([]map[string]interface{}, len(rows))
	for i, row := range rows {
		newRow := make(map[string]interface{}, len(row))
		for k, v := range row {
			// Only add qualified name if not already qualified.
			if !strings.Contains(k, ".") {
				newRow[tableName+"."+k] = v
			} else {
				newRow[k] = v
			}
		}
		result[i] = newRow
	}
	return result
}

// mergeRows combines a left row and a right row into a single merged row.
// Both inputs may already be qualified with table prefixes.
func mergeRows(left, right map[string]interface{}) map[string]interface{} {
	merged := make(map[string]interface{}, len(left)+len(right))
	for k, v := range left {
		merged[k] = v
	}
	for k, v := range right {
		merged[k] = v
	}
	return merged
}

// nullRow creates a row where all columns from rightCols under prefix are NULL.
// Used in OUTER JOINs where the right side has no matching row.
func nullRow(rightCols []string, prefix string) map[string]interface{} {
	row := make(map[string]interface{}, len(rightCols))
	for _, col := range rightCols {
		row[prefix+"."+col] = nil
	}
	return row
}

// ─── Join algorithms ──────────────────────────────────────────────────────────

// innerJoin returns only rows where the ON condition is TRUE.
// This is the most common join type — it is what most people mean when they
// write "JOIN" without a qualifier.
func innerJoin(left, right []map[string]interface{}, onExpr *parser.ASTNode) []map[string]interface{} {
	var result []map[string]interface{}
	for _, lRow := range left {
		for _, rRow := range right {
			merged := mergeRows(lRow, rRow)
			if onExpr == nil || isTruthy(evalExpr(onExpr, merged)) {
				result = append(result, merged)
			}
		}
	}
	return result
}

// leftJoin returns all left rows, with right columns set to NULL where there
// is no matching right row.
//
// Visual example:
//
//	employees (left):                departments (right):
//	id | name  | dept_id             id | name
//	---+-------+---------            ---+-------------
//	1  | Alice | 1                   1  | Engineering
//	2  | Bob   | 2                   2  | Marketing
//	4  | Dave  | NULL                (no dept 3)
//
//	LEFT JOIN result:
//	emp.id | emp.name | dept.id | dept.name
//	-------+----------+---------+-----------
//	1      | Alice    | 1       | Engineering
//	2      | Bob      | 2       | Marketing
//	4      | Dave     | NULL    | NULL          ← Dave has no dept, but still appears
func leftJoin(left, right []map[string]interface{}, onExpr *parser.ASTNode, rightCols []string, rightPrefix string) []map[string]interface{} {
	var result []map[string]interface{}
	for _, lRow := range left {
		matched := false
		for _, rRow := range right {
			merged := mergeRows(lRow, rRow)
			if onExpr == nil || isTruthy(evalExpr(onExpr, merged)) {
				result = append(result, merged)
				matched = true
			}
		}
		if !matched {
			// No right match: emit the left row with NULL right columns.
			nullRight := nullRow(rightCols, rightPrefix)
			result = append(result, mergeRows(lRow, nullRight))
		}
	}
	return result
}

// rightJoin is the mirror of leftJoin: all right rows are preserved, with
// left columns NULL for unmatched right rows.
func rightJoin(left, right []map[string]interface{}, onExpr *parser.ASTNode, rightCols []string, rightPrefix string) []map[string]interface{} {
	var result []map[string]interface{}

	// Collect which right rows were matched by some left row.
	matchedRight := make([]bool, len(right))

	for _, lRow := range left {
		for j, rRow := range right {
			merged := mergeRows(lRow, rRow)
			if onExpr == nil || isTruthy(evalExpr(onExpr, merged)) {
				result = append(result, merged)
				matchedRight[j] = true
			}
		}
	}

	// Emit unmatched right rows with NULL left columns.
	// We need to know the left column names to null them out.
	// We infer them from the first left row (if any).
	var leftNullRow map[string]interface{}
	if len(left) > 0 {
		leftNullRow = make(map[string]interface{}, len(left[0]))
		for k := range left[0] {
			leftNullRow[k] = nil
		}
	} else {
		leftNullRow = map[string]interface{}{}
	}

	for j, rRow := range right {
		if !matchedRight[j] {
			result = append(result, mergeRows(leftNullRow, rRow))
		}
	}

	return result
}

// fullOuterJoin combines left and right outer join: all rows from both sides
// are preserved, with NULLs for the missing side.
func fullOuterJoin(left, right []map[string]interface{}, onExpr *parser.ASTNode, rightCols []string, rightPrefix string) []map[string]interface{} {
	var result []map[string]interface{}

	matchedRight := make([]bool, len(right))

	// Process all left rows (like LEFT JOIN).
	for _, lRow := range left {
		matched := false
		for j, rRow := range right {
			merged := mergeRows(lRow, rRow)
			if onExpr == nil || isTruthy(evalExpr(onExpr, merged)) {
				result = append(result, merged)
				matched = true
				matchedRight[j] = true
			}
		}
		if !matched {
			nullRight := nullRow(rightCols, rightPrefix)
			result = append(result, mergeRows(lRow, nullRight))
		}
	}

	// Add unmatched right rows (like RIGHT JOIN).
	var leftNullRow map[string]interface{}
	if len(left) > 0 {
		leftNullRow = make(map[string]interface{}, len(left[0]))
		for k := range left[0] {
			leftNullRow[k] = nil
		}
	} else {
		leftNullRow = map[string]interface{}{}
	}

	for j, rRow := range right {
		if !matchedRight[j] {
			result = append(result, mergeRows(leftNullRow, rRow))
		}
	}

	return result
}

// crossJoin returns the Cartesian product of left and right.
// Every left row is combined with every right row. No ON condition.
//
// For N left rows and M right rows, this produces N×M output rows.
// Use with caution: two 1000-row tables produce 1,000,000 output rows.
func crossJoin(left, right []map[string]interface{}) []map[string]interface{} {
	result := make([]map[string]interface{}, 0, len(left)*len(right))
	for _, lRow := range left {
		for _, rRow := range right {
			result = append(result, mergeRows(lRow, rRow))
		}
	}
	return result
}
