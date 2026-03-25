// executor.go — The core SQL SELECT execution pipeline.
//
// This file implements the relational algebra pipeline that transforms a
// select_stmt AST node into a QueryResult by applying each clause in order.
//
// # Pipeline stages
//
// The stages are applied in the logical order defined by the SQL standard,
// which is different from the syntactic order in which you write SQL:
//
//	1. FROM      — Scan base table, build initial row contexts
//	2. JOIN      — Nested-loop join for each join_clause
//	3. WHERE     — Filter rows (σ operator in relational algebra)
//	4. GROUP BY  — Partition rows into groups (or implicit single group for aggregates)
//	5. HAVING    — Filter groups
//	6. SELECT    — Project expressions (π operator) and compute aggregates
//	7. DISTINCT  — Remove duplicate rows
//	8. ORDER BY  — Sort rows
//	9. LIMIT     — Slice the result set
//
// The SQL evaluation order is famously NOT the syntactic order. For example,
// GROUP BY is evaluated before SELECT, which means SELECT can reference group
// keys and aggregates, but individual row values are gone by SELECT time.
//
// # Row context
//
// Each row in the pipeline is a map[string]interface{} ("rowCtx").
// Keys are column names, possibly qualified with "tableName.colName".
// Values are SQL-typed: nil | int64 | float64 | string | bool.
//
// # Aggregate queries
//
// A query is an "aggregate query" if any of these conditions hold:
//   - It has a GROUP BY clause
//   - Its SELECT list contains aggregate function calls (COUNT, SUM, AVG, MIN, MAX)
//
// For aggregate queries, the SELECT stage operates on groups rather than
// individual rows. Non-aggregate select items must be GROUP BY keys.
package sqlengine

import (
	"fmt"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// executeSelect is the main entry point for SELECT execution.
// It orchestrates all pipeline stages and returns the final QueryResult.
func executeSelect(node *parser.ASTNode, source DataSource) (*QueryResult, error) {
	// ── Stage 1: FROM ────────────────────────────────────────────────────────
	//
	// Scan the base table and build initial row contexts.
	// Each row is a map[string]interface{} with column names as keys.
	rows, baseTableName, err := executeFrom(node, source)
	if err != nil {
		return nil, err
	}

	// ── Stage 2: JOIN ────────────────────────────────────────────────────────
	//
	// Apply each join_clause in order (left to right).
	// After joining, row contexts contain qualified column names.
	rows, err = executeJoins(node, rows, source, baseTableName)
	if err != nil {
		return nil, err
	}

	// ── Stage 3: WHERE ───────────────────────────────────────────────────────
	//
	// Filter rows where the WHERE predicate is TRUE.
	// NULL predicates also exclude the row (SQL three-valued logic).
	rows, err = executeWhere(node, rows)
	if err != nil {
		return nil, err
	}

	// ── Stage 4–6: GROUP BY / HAVING / SELECT ───────────────────────────────
	//
	// These three stages are tightly coupled because aggregate functions in
	// SELECT must be computed over groups. We handle them together.
	result, err := executeGroupAndProject(node, rows, source)
	if err != nil {
		return nil, err
	}

	// ── Stage 7: DISTINCT ────────────────────────────────────────────────────
	//
	// Remove duplicate rows if DISTINCT was specified.
	if isDistinct(node) {
		result.Rows = deduplicateRows(result.Rows)
	}

	// ── Stage 8: ORDER BY ───────────────────────────────────────────────────
	//
	// Sort the result rows by the ORDER BY expressions.
	result, err = executeOrderBy(node, result)
	if err != nil {
		return nil, err
	}

	// ── Stage 9: LIMIT / OFFSET ─────────────────────────────────────────────
	//
	// Slice the result to at most LIMIT rows, skipping OFFSET rows first.
	result = executeLimit(node, result)

	return result, nil
}

// ─── Stage 1: FROM ───────────────────────────────────────────────────────────

// executeFrom scans the base table referenced in the FROM clause.
//
// Grammar:
//   select_stmt = "SELECT" [...] select_list "FROM" table_ref { join_clause } ...
//   table_ref   = table_name [ "AS" NAME ]
//   table_name  = NAME [ "." NAME ]
//
// Returns:
//   - rows: slice of row maps (column name → value)
//   - tableName: the effective name/alias for qualifying columns in JOINs
//   - err: any scan or schema error
func executeFrom(node *parser.ASTNode, source DataSource) ([]map[string]interface{}, string, error) {
	tableRefNode := findChild(node, "table_ref")
	if tableRefNode == nil {
		return nil, "", fmt.Errorf("SELECT has no FROM clause")
	}

	tableName, alias := extractTableNameAndAlias(tableRefNode)
	rows, err := source.Scan(tableName)
	if err != nil {
		return nil, "", err
	}

	// Use alias as the prefix, or the real table name if no alias.
	effectiveName := alias

	// When there are JOINs, qualify column names with the table prefix.
	// This is required to distinguish "employees.id" from "departments.id".
	hasJoins := findChild(node, "join_clause") != nil
	if hasJoins {
		rows = qualifyRows(rows, effectiveName)
	}

	return rows, effectiveName, nil
}

// extractTableNameAndAlias extracts the table name and optional alias from a
// table_ref node.
//
// Grammar:
//   table_ref  = table_name [ "AS" NAME ]
//   table_name = NAME [ "." NAME ]
//
// Returns (tableName, alias). If no alias, alias == tableName.
func extractTableNameAndAlias(tableRefNode *parser.ASTNode) (string, string) {
	tableNameNode := findChild(tableRefNode, "table_name")
	if tableNameNode == nil {
		return "", ""
	}

	// Extract table name parts (handles schema.table qualified names).
	var nameParts []string
	for _, child := range tableNameNode.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if tok.Value != "." {
			nameParts = append(nameParts, tok.Value)
		}
	}

	tableName := strings.Join(nameParts, ".")

	// Check for optional alias: "AS" NAME
	alias := tableName
	foundAs := false
	for _, child := range tableRefNode.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if strings.ToUpper(tok.Value) == "AS" {
			foundAs = true
			continue
		}
		if foundAs && (tok.TypeName == "NAME" || tok.Type == lexer.TokenName) {
			alias = tok.Value
			break
		}
	}

	return tableName, alias
}

// ─── Stage 2: JOIN ───────────────────────────────────────────────────────────

// executeJoins applies each join_clause to the current row set in order.
//
// Multiple JOINs in a query are applied left to right:
//   FROM a JOIN b ON ... JOIN c ON ...
// first joins a with b, then joins the result with c.
func executeJoins(node *parser.ASTNode, rows []map[string]interface{}, source DataSource, baseTableName string) ([]map[string]interface{}, error) {
	// Collect all join_clause children.
	for _, child := range node.Children {
		joinNode, ok := child.(*parser.ASTNode)
		if !ok || joinNode.RuleName != "join_clause" {
			continue
		}

		var err error
		rows, err = applyJoin(rows, joinNode, source)
		if err != nil {
			return nil, err
		}
	}
	return rows, nil
}

// ─── Stage 3: WHERE ──────────────────────────────────────────────────────────

// executeWhere filters the row set using the WHERE predicate.
//
// Grammar: where_clause = "WHERE" expr
//
// SQL three-valued logic: only rows where the predicate evaluates to TRUE
// are kept. FALSE and NULL both cause the row to be excluded.
func executeWhere(node *parser.ASTNode, rows []map[string]interface{}) ([]map[string]interface{}, error) {
	whereNode := findChild(node, "where_clause")
	if whereNode == nil {
		return rows, nil // No WHERE clause: all rows pass
	}

	// Find the expression inside the WHERE clause.
	var exprNode *parser.ASTNode
	for _, child := range whereNode.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		exprNode = childNode
		break
	}

	if exprNode == nil {
		return rows, nil
	}

	var filtered []map[string]interface{}
	for _, row := range rows {
		val := evalExpr(exprNode, row)

		// Check for ColumnNotFoundError (signals missing column reference).
		if cnf, ok := val.(*columnNotFound); ok {
			return nil, &ColumnNotFoundError{ColumnName: cnf.name}
		}

		if isTruthy(val) {
			filtered = append(filtered, row)
		}
	}
	return filtered, nil
}

// ─── Stages 4–6: GROUP BY / HAVING / SELECT ──────────────────────────────────

// executeGroupAndProject handles GROUP BY, HAVING, and SELECT together.
//
// These stages are tightly coupled because:
//   1. SELECT expressions may contain aggregate functions (COUNT, SUM, etc.)
//   2. Aggregate functions must be computed per group
//   3. HAVING filters groups using the same aggregate results
//
// The processing branches based on whether the query is aggregate or not:
//   - Non-aggregate: project each row independently
//   - Aggregate: partition into groups, compute aggregates per group, project once per group
func executeGroupAndProject(node *parser.ASTNode, rows []map[string]interface{}, source DataSource) (*QueryResult, error) {
	selectListNode := findChild(node, "select_list")
	if selectListNode == nil {
		return &QueryResult{}, nil
	}

	// Determine if this is an aggregate query.
	groupNode := findChild(node, "group_clause")
	isAggregate := groupNode != nil || selectListHasAggregate(selectListNode)

	if isAggregate {
		return executeAggregateQuery(node, rows, selectListNode, source)
	}

	return executeProjection(rows, selectListNode, source)
}

// selectListHasAggregate returns true if any select_item contains an aggregate
// function call. This detects queries like "SELECT COUNT(*) FROM t" which are
// aggregate even without GROUP BY.
func selectListHasAggregate(selectListNode *parser.ASTNode) bool {
	for _, child := range selectListNode.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == "select_item" {
			for _, itemChild := range childNode.Children {
				if itemNode, ok := itemChild.(*parser.ASTNode); ok {
					if hasAggregateInExpr(itemNode) {
						return true
					}
				}
			}
		}
	}
	return false
}

// ─── Non-aggregate projection ─────────────────────────────────────────────────

// executeProjection handles SELECT for non-aggregate queries.
// Each row is projected independently: expressions are evaluated per row.
func executeProjection(rows []map[string]interface{}, selectListNode *parser.ASTNode, source DataSource) (*QueryResult, error) {
	// Check for SELECT *
	isStar := false
	for _, child := range selectListNode.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.Value == "*" {
				isStar = true
				break
			}
		}
	}

	if isStar {
		return projectStar(rows)
	}

	// Collect select_item nodes.
	items := collectRuleChildren(selectListNode, "select_item")

	// Build column names from select items.
	columns, err := buildColumnNames(items, rows)
	if err != nil {
		return nil, err
	}

	// Project each row.
	resultRows := make([][]interface{}, 0, len(rows))
	for _, row := range rows {
		projected, err := projectRow(items, row)
		if err != nil {
			return nil, err
		}
		resultRows = append(resultRows, projected)
	}

	return &QueryResult{Columns: columns, Rows: resultRows}, nil
}

// projectStar handles SELECT * by returning all columns in schema order.
// We use the row keys sorted for deterministic output.
func projectStar(rows []map[string]interface{}) (*QueryResult, error) {
	if len(rows) == 0 {
		return &QueryResult{Columns: []string{}, Rows: [][]interface{}{}}, nil
	}

	// Build sorted column list from first row keys.
	// For qualified names (from JOINs), sort puts "table.col" in alphabetical order.
	first := rows[0]
	columns := make([]string, 0, len(first))
	for k := range first {
		columns = append(columns, k)
	}
	sort.Strings(columns)

	resultRows := make([][]interface{}, len(rows))
	for i, row := range rows {
		r := make([]interface{}, len(columns))
		for j, col := range columns {
			r[j] = row[col]
		}
		resultRows[i] = r
	}

	return &QueryResult{Columns: columns, Rows: resultRows}, nil
}

// buildColumnNames derives output column names from select_item nodes.
// For "expr AS alias", the alias is used. Otherwise we derive a name from
// the expression (column reference name, or "expr_N" for complex expressions).
func buildColumnNames(items []*parser.ASTNode, rows []map[string]interface{}) ([]string, error) {
	columns := make([]string, len(items))
	for i, item := range items {
		columns[i] = inferColumnName(item, i)
	}
	return columns, nil
}

// inferColumnName determines the output column name for a select_item.
//
// Priority:
//  1. Explicit alias: "expr AS alias" → "alias"
//  2. Column reference: "table.col" → "col" (unqualified)
//  3. Function call: "COUNT(*)" → "COUNT(*)"
//  4. Fallback: "col_N"
func inferColumnName(item *parser.ASTNode, index int) string {
	// Check for explicit AS alias.
	// Grammar: select_item = expr [ "AS" NAME ]
	foundAs := false
	for _, child := range item.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if strings.ToUpper(tok.Value) == "AS" {
			foundAs = true
			continue
		}
		if foundAs && (tok.TypeName == "NAME" || tok.Type == lexer.TokenName) {
			return tok.Value
		}
	}

	// No alias: try to derive from the expression.
	for _, child := range item.Children {
		exprNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		name := deriveExprName(exprNode)
		if name != "" {
			return name
		}
	}

	return fmt.Sprintf("col_%d", index+1)
}

// deriveExprName recursively derives a name from an expression AST.
// For column references, returns the unqualified column name.
// For function calls, returns "funcName(...)".
func deriveExprName(node *parser.ASTNode) string {
	if node == nil {
		return ""
	}

	switch node.RuleName {
	case "column_ref":
		// Return the last NAME token (unqualified column name).
		var lastName string
		for _, child := range node.Children {
			tok, ok := child.(lexer.Token)
			if !ok {
				continue
			}
			if tok.Value != "." {
				lastName = tok.Value
			}
		}
		return lastName

	case "function_call":
		// Return "FUNCNAME(*)" or "FUNCNAME(arg)".
		var fnName string
		for _, child := range node.Children {
			tok, ok := child.(lexer.Token)
			if !ok {
				continue
			}
			if tok.TypeName == "NAME" || tok.Type == lexer.TokenName {
				fnName = strings.ToUpper(tok.Value)
				break
			}
		}
		if fnName != "" {
			return fnName + "(*)"
		}
	}

	// Recurse into wrapper nodes (expr, or_expr, and_expr, etc.).
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		name := deriveExprName(childNode)
		if name != "" {
			return name
		}
	}

	return ""
}

// projectRow evaluates all select_item expressions for one row.
func projectRow(items []*parser.ASTNode, row map[string]interface{}) ([]interface{}, error) {
	result := make([]interface{}, len(items))
	for i, item := range items {
		val, err := evalSelectItem(item, row)
		if err != nil {
			return nil, err
		}
		result[i] = val
	}
	return result, nil
}

// evalSelectItem evaluates the expression part of a select_item.
// Grammar: select_item = expr [ "AS" NAME ]
// We evaluate the expr child and ignore the alias (already used for column name).
func evalSelectItem(item *parser.ASTNode, row map[string]interface{}) (interface{}, error) {
	for _, child := range item.Children {
		exprNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		val := evalExpr(exprNode, row)
		if cnf, ok := val.(*columnNotFound); ok {
			return nil, &ColumnNotFoundError{ColumnName: cnf.name}
		}
		return val, nil
	}
	return nil, nil
}

// ─── Aggregate query execution ────────────────────────────────────────────────

// executeAggregateQuery handles GROUP BY + HAVING + SELECT for aggregate queries.
//
// Processing steps:
//  1. Parse the GROUP BY key expressions
//  2. Partition rows into groups by GROUP BY key
//  3. For each group, compute aggregate function results
//  4. Apply HAVING filter (using aggregate results)
//  5. Project the select list (one output row per group)
func executeAggregateQuery(node *parser.ASTNode, rows []map[string]interface{}, selectListNode *parser.ASTNode, source DataSource) (*QueryResult, error) {
	// Parse GROUP BY keys.
	var groupKeys []*parser.ASTNode
	if groupNode := findChild(node, "group_clause"); groupNode != nil {
		groupKeys = collectGroupByKeys(groupNode)
	}

	// Partition rows into groups.
	// Each group is identified by a string key derived from the GROUP BY expressions.
	// We preserve insertion order using a slice of keys.
	type group struct {
		keyStr string
		keyRow map[string]interface{} // representative row for key values
		rows   []map[string]interface{}
	}

	var groupOrder []string
	groups := make(map[string]*group)

	for _, row := range rows {
		keyStr := computeGroupKey(groupKeys, row)
		if _, exists := groups[keyStr]; !exists {
			groupOrder = append(groupOrder, keyStr)
			groups[keyStr] = &group{
				keyStr: keyStr,
				keyRow: row,
				rows:   nil,
			}
		}
		groups[keyStr].rows = append(groups[keyStr].rows, row)
	}

	// If no rows and no groups, still produce one group for aggregate-only queries
	// like "SELECT COUNT(*) FROM empty_table" which should return 0, not no rows.
	if len(groups) == 0 && len(groupKeys) == 0 {
		groupOrder = []string{""}
		groups[""] = &group{keyStr: "", keyRow: map[string]interface{}{}, rows: nil}
	}

	// Build HAVING clause (if any).
	havingNode := findChild(node, "having_clause")

	// Collect select_item nodes.
	isStar := false
	for _, child := range selectListNode.Children {
		if tok, ok := child.(lexer.Token); ok && tok.Value == "*" {
			isStar = true
			break
		}
	}

	var items []*parser.ASTNode
	if !isStar {
		items = collectRuleChildren(selectListNode, "select_item")
	}

	// Build column names.
	var columns []string
	if isStar {
		columns = []string{"*"}
	} else {
		var err error
		columns, err = buildColumnNames(items, rows)
		if err != nil {
			return nil, err
		}
	}

	// Process each group.
	var resultRows [][]interface{}
	for _, keyStr := range groupOrder {
		g := groups[keyStr]

		// Compute aggregate values for this group.
		// We scan the select list to find all aggregate calls and pre-compute them.
		aggCtx := computeGroupAggregates(items, g.rows)

		// Build a merged row context: group key values + aggregate results.
		// This is what HAVING and SELECT expressions will evaluate against.
		groupCtx := make(map[string]interface{})
		// Copy the representative row's values (for GROUP BY key access).
		for k, v := range g.keyRow {
			groupCtx[k] = v
		}
		// Overlay aggregate results.
		for k, v := range aggCtx {
			groupCtx[k] = v
		}

		// Apply HAVING filter.
		if havingNode != nil {
			passed := applyHaving(havingNode, items, g.rows, groupCtx)
			if !passed {
				continue
			}
		}

		// Project the select list for this group.
		if isStar {
			// SELECT * with GROUP BY: return the representative row.
			row, err := projectStar([]map[string]interface{}{g.keyRow})
			if err != nil {
				return nil, err
			}
			if len(row.Rows) > 0 {
				resultRows = append(resultRows, row.Rows[0])
			}
		} else {
			projected, err := projectAggregateRow(items, g.rows, groupCtx)
			if err != nil {
				return nil, err
			}
			resultRows = append(resultRows, projected)
		}
	}

	if isStar && len(resultRows) > 0 {
		// Derive columns from the first result row.
		if len(rows) > 0 {
			starResult, err := projectStar(rows[:1])
			if err == nil {
				columns = starResult.Columns
			}
		}
	}

	return &QueryResult{Columns: columns, Rows: resultRows}, nil
}

// collectGroupByKeys extracts the column_ref nodes from a group_clause.
// Grammar: group_clause = "GROUP" "BY" column_ref { "," column_ref }
func collectGroupByKeys(groupNode *parser.ASTNode) []*parser.ASTNode {
	return collectRuleChildren(groupNode, "column_ref")
}

// computeGroupKey computes a string key for a row based on the GROUP BY expressions.
// Two rows with the same group key will be in the same group.
//
// The key format is: "val1|val2|val3" where each val is the string representation
// of the corresponding GROUP BY expression evaluated against the row.
func computeGroupKey(keys []*parser.ASTNode, row map[string]interface{}) string {
	if len(keys) == 0 {
		return "" // All rows in one group (aggregate without GROUP BY)
	}
	parts := make([]string, len(keys))
	for i, key := range keys {
		val := evalExpr(key, row)
		if val == nil {
			parts[i] = "\x00NULL\x00" // distinct sentinel for NULL grouping
		} else {
			parts[i] = fmt.Sprintf("%v", val)
		}
	}
	return strings.Join(parts, "\x00|\x00")
}

// computeGroupAggregates scans the select list, finds all aggregate function
// calls, and computes their results over the given group rows.
//
// Returns a map of aggregate key → value that is merged into the group context.
// Keys use a format like "_agg_COUNT_0" to be unique per aggregate call.
func computeGroupAggregates(items []*parser.ASTNode, groupRows []map[string]interface{}) map[string]interface{} {
	aggCtx := make(map[string]interface{})
	for i, item := range items {
		_ = i
		extractAggregates(item, groupRows, aggCtx)
	}
	return aggCtx
}

// extractAggregates recursively finds function_call nodes in the expression
// tree and computes their aggregate results.
func extractAggregates(node *parser.ASTNode, rows []map[string]interface{}, aggCtx map[string]interface{}) {
	if node == nil {
		return
	}
	if node.RuleName == "function_call" {
		fnName, argNode, isStar := parseFunctionCall(node)
		if isAggregateFunction(fnName) {
			// Use a key based on the function name + arg for caching.
			key := aggKeyFor(node)
			if _, exists := aggCtx[key]; !exists {
				aggCtx[key] = computeAggregate(fnName, argNode, isStar, rows)
			}
			// Also store under generic "COUNT" key for simple lookups.
			aggCtx["_agg_"+fnName] = aggCtx[key]
		}
		return
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			extractAggregates(childNode, rows, aggCtx)
		}
	}
}

// aggKeyFor builds a stable string key for a function_call node.
// This ensures that "COUNT(*)" always maps to the same cache key.
func aggKeyFor(node *parser.ASTNode) string {
	fnName, _, isStar := parseFunctionCall(node)
	if isStar {
		return "_agg_" + fnName + "_STAR"
	}
	return "_agg_" + fnName
}

// parseFunctionCall extracts the function name, argument node, and star flag
// from a function_call AST node.
//
// Grammar: function_call = NAME "(" ( STAR | [ value_list ] ) ")"
func parseFunctionCall(node *parser.ASTNode) (fnName string, argNode *parser.ASTNode, isStar bool) {
	for _, child := range node.Children {
		switch v := child.(type) {
		case lexer.Token:
			if v.TypeName == "NAME" || v.Type == lexer.TokenName {
				fnName = strings.ToUpper(v.Value)
			}
			if v.Value == "*" {
				isStar = true
			}
		case *parser.ASTNode:
			if v.RuleName == "value_list" {
				// The argument is the first expression in the value list.
				for _, vChild := range v.Children {
					if vNode, ok := vChild.(*parser.ASTNode); ok {
						argNode = vNode
						break
					}
				}
			}
		}
	}
	return
}

// applyHaving evaluates the HAVING predicate for one group.
// Grammar: having_clause = "HAVING" expr
func applyHaving(havingNode *parser.ASTNode, items []*parser.ASTNode, groupRows []map[string]interface{}, groupCtx map[string]interface{}) bool {
	// Re-compute aggregates into groupCtx if not already done.
	// (They should already be there from computeGroupAggregates.)

	var exprNode *parser.ASTNode
	for _, child := range havingNode.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			exprNode = childNode
			break
		}
	}

	if exprNode == nil {
		return true
	}

	// Evaluate the HAVING expression with a special evaluator that handles
	// aggregate function calls by looking them up in groupCtx.
	val := evalExprWithAggCtx(exprNode, groupCtx, groupRows)
	return isTruthy(val)
}

// evalExprWithAggCtx evaluates an expression in an aggregate context.
// When it encounters a function_call node, it computes the aggregate over
// groupRows rather than delegating to the scalar evaluator.
func evalExprWithAggCtx(node *parser.ASTNode, groupCtx map[string]interface{}, groupRows []map[string]interface{}) interface{} {
	if node == nil {
		return nil
	}

	if node.RuleName == "function_call" {
		fnName, argNode, isStar := parseFunctionCall(node)
		if isAggregateFunction(fnName) {
			key := aggKeyFor(node)
			if val, ok := groupCtx[key]; ok {
				return val
			}
			// Compute on demand.
			val := computeAggregate(fnName, argNode, isStar, groupRows)
			groupCtx[key] = val
			groupCtx["_agg_"+fnName] = val
			return val
		}
		return nil
	}

	// For other nodes, use the standard evaluator (which has groupCtx).
	return evalExprWithGroupCtx(node, groupCtx, groupRows)
}

// evalExprWithGroupCtx is like evalExpr but intercepts function_call nodes
// to compute aggregates over the group rows.
func evalExprWithGroupCtx(node *parser.ASTNode, groupCtx map[string]interface{}, groupRows []map[string]interface{}) interface{} {
	if node == nil {
		return nil
	}

	switch node.RuleName {
	case "function_call":
		return evalExprWithAggCtx(node, groupCtx, groupRows)
	case "expr", "or_expr", "and_expr", "not_expr", "comparison",
		"additive", "multiplicative", "unary", "primary", "column_ref":
		// These are handled by rebuilding the call with our intercepting evaluator.
		return evalExprIntercepted(node, groupCtx, groupRows)
	default:
		return evalExpr(node, groupCtx)
	}
}

// evalExprIntercepted evaluates an expression, replacing function_call
// evaluation with aggregate computation over groupRows.
// This is needed so HAVING COUNT(*) > 5 works correctly.
func evalExprIntercepted(node *parser.ASTNode, groupCtx map[string]interface{}, groupRows []map[string]interface{}) interface{} {
	if node == nil {
		return nil
	}

	// Intercept function_call nodes.
	if node.RuleName == "function_call" {
		return evalExprWithAggCtx(node, groupCtx, groupRows)
	}

	// For primary nodes, check if any child is a function_call.
	if node.RuleName == "primary" {
		for _, child := range node.Children {
			if childNode, ok := child.(*parser.ASTNode); ok {
				if childNode.RuleName == "function_call" {
					return evalExprWithAggCtx(childNode, groupCtx, groupRows)
				}
			}
		}
	}

	// Fall through to standard evaluation.
	return evalExpr(node, groupCtx)
}

// projectAggregateRow projects one output row for an aggregate group.
// For each select_item, aggregate function calls are computed from groupRows,
// and non-aggregate expressions are evaluated against the group context.
func projectAggregateRow(items []*parser.ASTNode, groupRows []map[string]interface{}, groupCtx map[string]interface{}) ([]interface{}, error) {
	result := make([]interface{}, len(items))
	for i, item := range items {
		// Find the expression child (skip "AS alias" tokens).
		for _, child := range item.Children {
			exprNode, ok := child.(*parser.ASTNode)
			if !ok {
				continue
			}
			val := evalExprWithGroupCtx(exprNode, groupCtx, groupRows)
			if cnf, ok := val.(*columnNotFound); ok {
				return nil, &ColumnNotFoundError{ColumnName: cnf.name}
			}
			result[i] = val
			break
		}
	}
	return result, nil
}

// ─── Stage 7: DISTINCT ───────────────────────────────────────────────────────

// isDistinct returns true if the SELECT has a DISTINCT modifier.
// Grammar: select_stmt = "SELECT" [ "DISTINCT" | "ALL" ] select_list ...
func isDistinct(node *parser.ASTNode) bool {
	for _, child := range node.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if strings.ToUpper(tok.Value) == "DISTINCT" {
			return true
		}
	}
	return false
}

// deduplicateRows removes duplicate rows from the result set.
// Two rows are considered duplicate if all their values are equal.
// We use a string serialization as the dedup key.
func deduplicateRows(rows [][]interface{}) [][]interface{} {
	seen := make(map[string]bool)
	var result [][]interface{}
	for _, row := range rows {
		key := rowKey(row)
		if !seen[key] {
			seen[key] = true
			result = append(result, row)
		}
	}
	return result
}

// rowKey serializes a row to a string for deduplication and grouping.
func rowKey(row []interface{}) string {
	parts := make([]string, len(row))
	for i, v := range row {
		if v == nil {
			parts[i] = "\x00NULL\x00"
		} else {
			parts[i] = fmt.Sprintf("%T:%v", v, v)
		}
	}
	return strings.Join(parts, "\x00")
}

// ─── Stage 8: ORDER BY ───────────────────────────────────────────────────────

// executeOrderBy sorts the result rows by the ORDER BY expressions.
//
// Grammar:
//   order_clause = "ORDER" "BY" order_item { "," order_item }
//   order_item   = expr [ "ASC" | "DESC" ]
//
// We re-evaluate the ORDER BY expressions against the original row set to
// get sort keys. Each order_item has an optional direction (default ASC).
//
// Implementation note: we sort the result rows by re-evaluating the sort key
// expressions against... but wait, after projection the row context is gone.
// So we need to sort BEFORE projection, or carry the sort keys through.
//
// Current approach: sort the result rows using column index references for
// simple cases (column name matches an output column), or re-evaluate
// the sort key against the first matching pre-projection row.
//
// For simplicity, this implementation sorts by output column value when the
// ORDER BY expression is a simple column reference.
func executeOrderBy(node *parser.ASTNode, result *QueryResult) (*QueryResult, error) {
	orderNode := findChild(node, "order_clause")
	if orderNode == nil {
		return result, nil
	}

	// Collect order_item nodes.
	items := collectRuleChildren(orderNode, "order_item")
	if len(items) == 0 {
		return result, nil
	}

	// Build sort key descriptors.
	type sortKey struct {
		colIndex int    // index in Columns (-1 if not found)
		colName  string // for debugging
		desc     bool   // true = DESC
	}

	keys := make([]sortKey, 0, len(items))
	for _, item := range items {
		key := sortKey{colIndex: -1}

		// Determine direction.
		for _, child := range item.Children {
			tok, ok := child.(lexer.Token)
			if !ok {
				continue
			}
			if strings.ToUpper(tok.Value) == "DESC" {
				key.desc = true
			}
		}

		// Find the column name from the expression.
		for _, child := range item.Children {
			exprNode, ok := child.(*parser.ASTNode)
			if !ok {
				continue
			}
			// Try to resolve as a column reference.
			colName := deriveExprName(exprNode)
			if colName == "" {
				// Try deeper.
				colName = deriveExprName(findChildDeep(exprNode, "column_ref"))
			}
			key.colName = colName

			// Find the column index in the result.
			for i, col := range result.Columns {
				if strings.EqualFold(col, colName) {
					key.colIndex = i
					break
				}
			}
			break
		}

		keys = append(keys, key)
	}

	// Sort the rows using a stable sort so that equal rows preserve their
	// original order (important for LIMIT + ORDER BY queries).
	sort.SliceStable(result.Rows, func(i, j int) bool {
		for _, key := range keys {
			if key.colIndex < 0 {
				continue
			}
			a := result.Rows[i][key.colIndex]
			b := result.Rows[j][key.colIndex]

			// NULLs sort last (NULLS LAST is the PostgreSQL default).
			if a == nil && b == nil {
				continue
			}
			if a == nil {
				return false // nil sorts after non-nil
			}
			if b == nil {
				return true
			}

			cmp := compareValues(a, b)
			if cmp == 0 {
				continue // equal: try next sort key
			}
			if key.desc {
				return cmp > 0
			}
			return cmp < 0
		}
		return false
	})

	return result, nil
}

// ─── Stage 9: LIMIT / OFFSET ─────────────────────────────────────────────────

// executeLimit applies LIMIT and OFFSET to the result set.
//
// Grammar: limit_clause = "LIMIT" NUMBER [ "OFFSET" NUMBER ]
//
// OFFSET skips that many rows from the start.
// LIMIT caps the number of rows returned after OFFSET is applied.
func executeLimit(node *parser.ASTNode, result *QueryResult) *QueryResult {
	limitNode := findChild(node, "limit_clause")
	if limitNode == nil {
		return result
	}

	var limitVal, offsetVal int64
	limitVal = -1 // -1 means no limit

	// Parse LIMIT and OFFSET values from the NUMBER tokens.
	// Grammar: "LIMIT" NUMBER [ "OFFSET" NUMBER ]
	foundLimit := false
	foundOffset := false
	for _, child := range limitNode.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		switch {
		case strings.ToUpper(tok.Value) == "LIMIT":
			foundLimit = true
		case strings.ToUpper(tok.Value) == "OFFSET":
			foundOffset = true
			foundLimit = false // reset so next NUMBER goes to offset
		case tok.TypeName == "NUMBER" || tok.Type == lexer.TokenNumber:
			n := parseNumber(tok.Value)
			if nInt, ok := n.(int64); ok {
				if foundLimit && limitVal == -1 {
					limitVal = nInt
					foundLimit = false
				} else if foundOffset {
					offsetVal = nInt
					foundOffset = false
				}
			}
		}
	}

	rows := result.Rows

	// Apply OFFSET.
	// Compare against int64(len(rows)) to avoid narrowing int64→int on 32-bit
	// platforms (CWE-190). len() always fits in int64 since slice length is
	// bounded by addressable memory.
	if offsetVal > 0 {
		if offsetVal >= int64(len(rows)) {
			rows = nil
		} else {
			rows = rows[offsetVal:]
		}
	}

	// Apply LIMIT.
	// Same int64 comparison to avoid narrowing conversion (CWE-190).
	if limitVal >= 0 && limitVal < int64(len(rows)) {
		rows = rows[:limitVal]
	}

	return &QueryResult{Columns: result.Columns, Rows: rows}
}
