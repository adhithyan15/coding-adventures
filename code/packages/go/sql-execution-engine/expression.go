// SQL expression evaluator for the sql-execution-engine.
//
// This file implements evalExpr, the recursive evaluator that takes an AST
// node representing a SQL expression and a row context (map from column name
// to value), and returns the computed SQL value.
//
// # The expression grammar hierarchy
//
// The grammar defines expression precedence through chained rules. Higher in
// the list = lower precedence (evaluated last); lower in the list = higher
// precedence (evaluated first).
//
//	expr → or_expr           (lowest precedence)
//	     → and_expr
//	     → not_expr
//	     → comparison        (=, !=, <, >, <=, >=, BETWEEN, IN, LIKE, IS NULL)
//	     → additive          (+, -)
//	     → multiplicative    (*, /, %)
//	     → unary             (unary minus)
//	     → primary           (literals, column refs, function calls, parens)
//
// This chain ensures that "a + b > c AND d = e" is parsed as
// "((a + b) > c) AND (d = e)" — arithmetic before comparison before logic.
//
// # Three-valued (NULL) logic
//
// SQL uses three-valued logic: TRUE, FALSE, and NULL (unknown). The key rules:
//
//	NULL AND TRUE  = NULL    NULL OR TRUE  = TRUE
//	NULL AND FALSE = FALSE   NULL OR FALSE = NULL
//	NULL AND NULL  = NULL    NULL OR NULL  = NULL
//	NOT NULL = NULL
//
// The WHERE clause only passes rows where the predicate is TRUE (not FALSE,
// not NULL).
//
// # Row context format
//
// rowCtx is a map[string]interface{} with two key formats:
//
//  1. Single-table queries: keys are plain column names ("id", "name")
//  2. Multi-table (JOIN) queries: keys are qualified names ("employees.id",
//     "departments.name") AND plain names for the first/alias match.
//
// The column resolver tries qualified lookup first, then plain lookup.
package sqlengine

import (
	"fmt"
	"math"
	"strconv"
	"strings"
	"unicode"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// evalExpr evaluates a SQL expression AST node against a row context.
//
// The rowCtx maps column names (and qualified "table.col" names) to their
// current values. Returns nil for SQL NULL, or a Go value (int64, float64,
// string, bool).
//
// This function is recursive: complex expressions like "a + b > c OR d IS NULL"
// decompose into nested evalExpr calls following the grammar structure.
func evalExpr(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	if node == nil {
		return nil
	}

	switch node.RuleName {
	case "expr":
		// expr is a single-rule wrapper: expr = or_expr
		// It exists to give the top-level expression a stable name.
		if len(node.Children) > 0 {
			if child, ok := node.Children[0].(*parser.ASTNode); ok {
				return evalExpr(child, rowCtx)
			}
		}
		return nil

	case "or_expr":
		return evalOrExpr(node, rowCtx)

	case "and_expr":
		return evalAndExpr(node, rowCtx)

	case "not_expr":
		return evalNotExpr(node, rowCtx)

	case "comparison":
		return evalComparison(node, rowCtx)

	case "additive":
		return evalAdditive(node, rowCtx)

	case "multiplicative":
		return evalMultiplicative(node, rowCtx)

	case "unary":
		return evalUnary(node, rowCtx)

	case "primary":
		return evalPrimary(node, rowCtx)

	case "column_ref":
		return evalColumnRef(node, rowCtx)

	case "function_call":
		// function_call inside evalExpr means a scalar context (no GROUP BY).
		// For scalar COUNT(*) of all rows, this is handled by the executor.
		// Here we return nil — aggregate functions in non-aggregate contexts
		// should not be evaluated here (they are pre-computed by the executor).
		return nil

	default:
		return nil
	}
}

// ─── OR ──────────────────────────────────────────────────────────────────────

// evalOrExpr handles: or_expr = and_expr { "OR" and_expr }
//
// Grammar structure in Children:
//   - First child: and_expr node
//   - Subsequent pairs: KEYWORD("OR"), and_expr node
//
// Three-valued OR truth table (SQL standard):
//
//	T OR T = T    T OR F = T    T OR N = T
//	F OR T = T    F OR F = F    F OR N = N
//	N OR T = T    N OR F = N    N OR N = N
func evalOrExpr(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	// Collect all the and_expr operands (skip "OR" keyword tokens).
	operands := collectRuleChildren(node, "and_expr")
	if len(operands) == 0 {
		return nil
	}

	result := evalExpr(operands[0], rowCtx)
	// Propagate errors immediately.
	if _, ok := result.(*columnNotFound); ok {
		return result
	}
	for _, operand := range operands[1:] {
		right := evalExpr(operand, rowCtx)
		if _, ok := right.(*columnNotFound); ok {
			return right
		}
		result = sqlOr(result, right)
	}
	return result
}

// evalAndExpr handles: and_expr = not_expr { "AND" not_expr }
//
// Three-valued AND truth table (SQL standard):
//
//	T AND T = T    T AND F = F    T AND N = N
//	F AND T = F    F AND F = F    F AND N = F
//	N AND T = N    N AND F = F    N AND N = N
func evalAndExpr(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	operands := collectRuleChildren(node, "not_expr")
	if len(operands) == 0 {
		return nil
	}

	result := evalExpr(operands[0], rowCtx)
	// Propagate errors immediately.
	if _, ok := result.(*columnNotFound); ok {
		return result
	}
	for _, operand := range operands[1:] {
		right := evalExpr(operand, rowCtx)
		if _, ok := right.(*columnNotFound); ok {
			return right
		}
		result = sqlAnd(result, right)
	}
	return result
}

// evalNotExpr handles: not_expr = "NOT" not_expr | comparison
//
// NOT has special NULL behavior:
//   - NOT TRUE  = FALSE
//   - NOT FALSE = TRUE
//   - NOT NULL  = NULL  (unknown negated is still unknown)
func evalNotExpr(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	// Check if this is "NOT <expr>" or just a passthrough to comparison.
	hasNot := false
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.Value == "NOT" {
				hasNot = true
			}
		}
	}

	if !hasNot {
		// Passthrough: not_expr = comparison
		for _, child := range node.Children {
			if childNode, ok := child.(*parser.ASTNode); ok {
				return evalExpr(childNode, rowCtx)
			}
		}
		return nil
	}

	// "NOT" not_expr: evaluate the inner not_expr and negate.
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		val := evalExpr(childNode, rowCtx)
		if val == nil {
			return nil // NOT NULL = NULL
		}
		if b, ok := val.(bool); ok {
			return !b
		}
		// Non-boolean truthy values: treat non-nil as true.
		return false
	}
	return nil
}

// ─── Comparison ──────────────────────────────────────────────────────────────

// evalComparison handles the comparison rule:
//
//	comparison = additive [ cmp_op additive
//	           | "BETWEEN" additive "AND" additive
//	           | "NOT" "BETWEEN" additive "AND" additive
//	           | "IN" "(" value_list ")"
//	           | "NOT" "IN" "(" value_list ")"
//	           | "LIKE" additive
//	           | "NOT" "LIKE" additive
//	           | "IS" "NULL"
//	           | "IS" "NOT" "NULL" ]
//
// The comparison is optional — if there's no operator, comparison just
// delegates to the left-hand additive expression.
//
// IMPORTANT: The grammar puts cmp_op, IS, BETWEEN, IN, LIKE etc. as
// ASTNode children (not raw tokens), except for keyword operators like
// IS, BETWEEN, NOT, IN, LIKE which are literal token matches that appear
// as lexer.Token in the children list. The cmp_op rule wraps the operator
// token in a node.
func evalComparison(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	// Separate children into ASTNodes and Tokens.
	// The first ASTNode child is always the lhs additive expression.
	// Subsequent children may be:
	//   - cmp_op ASTNode (for =, !=, <, >, <=, >=)
	//   - keyword tokens (BETWEEN, IS, IN, LIKE, NOT, NULL, AND)
	//   - value_list ASTNode (for IN)
	//   - more additive ASTNodes (rhs)
	var nodes []*parser.ASTNode
	var tokens []lexer.Token
	for _, c := range node.Children {
		switch v := c.(type) {
		case *parser.ASTNode:
			nodes = append(nodes, v)
		case lexer.Token:
			tokens = append(tokens, v)
		}
	}

	// The left-hand side is always the first ASTNode child.
	if len(nodes) == 0 {
		return nil
	}

	lhs := evalExpr(nodes[0], rowCtx)

	// Propagate columnNotFound errors immediately.
	if _, ok := lhs.(*columnNotFound); ok {
		return lhs
	}

	// Check for cmp_op node (standard comparison operators: =, !=, <, >, <=, >=).
	// cmp_op is always a child ASTNode, never a bare token.
	cmpOpNode := findChild(node, "cmp_op")
	if cmpOpNode != nil {
		op := extractCmpOp(cmpOpNode)
		// The rhs is the next non-cmp_op ASTNode after the lhs.
		var rhs interface{}
		for i, n := range nodes {
			if i == 0 {
				continue // skip lhs
			}
			if n.RuleName != "cmp_op" {
				rhs = evalExpr(n, rowCtx)
				break
			}
		}
		return evalBinaryComparison(lhs, op, rhs)
	}

	// No cmp_op and no tokens: plain value passthrough.
	if len(tokens) == 0 {
		return lhs
	}

	// Keyword-based operators: BETWEEN, IS, IN, LIKE, NOT (before IN/BETWEEN/LIKE).
	return evalComparisonKeywordOp(lhs, nodes, tokens, rowCtx)
}

// evalComparisonKeywordOp handles keyword-based comparison operators:
// IS NULL, IS NOT NULL, BETWEEN, NOT BETWEEN, IN, NOT IN, LIKE, NOT LIKE.
// This is called when no cmp_op child node was found (so not =, !=, <, >, <=, >=).
func evalComparisonKeywordOp(lhs interface{}, nodes []*parser.ASTNode, tokens []lexer.Token, rowCtx map[string]interface{}) interface{} {
	if len(tokens) == 0 {
		return lhs
	}

	// Propagate columnNotFound errors before any operator logic.
	if _, ok := lhs.(*columnNotFound); ok {
		return lhs
	}

	// Determine operator by looking at first keyword token.
	firstTok := strings.ToUpper(tokens[0].Value)

	switch {
	case firstTok == "IS" && len(tokens) >= 2 && strings.ToUpper(tokens[1].Value) == "NULL":
		// IS NULL: true if lhs is nil
		return lhs == nil

	case firstTok == "IS" && len(tokens) >= 3 &&
		strings.ToUpper(tokens[1].Value) == "NOT" &&
		strings.ToUpper(tokens[2].Value) == "NULL":
		// IS NOT NULL: true if lhs is not nil
		return lhs != nil

	case firstTok == "BETWEEN":
		// BETWEEN low AND high: equivalent to lhs >= low AND lhs <= high
		// Grammar: "BETWEEN" additive "AND" additive
		// The "AND" here is a literal token, not the logical AND operator.
		if len(nodes) >= 3 {
			low := evalExpr(nodes[1], rowCtx)
			high := evalExpr(nodes[2], rowCtx)
			return evalBetween(lhs, low, high)
		}
		return nil

	case firstTok == "NOT" && len(tokens) >= 2 && strings.ToUpper(tokens[1].Value) == "BETWEEN":
		// NOT BETWEEN low AND high
		if len(nodes) >= 3 {
			low := evalExpr(nodes[1], rowCtx)
			high := evalExpr(nodes[2], rowCtx)
			result := evalBetween(lhs, low, high)
			if result == nil {
				return nil
			}
			if b, ok := result.(bool); ok {
				return !b
			}
			return nil
		}
		return nil

	case firstTok == "IN":
		// IN (value_list): true if lhs equals any value in the list
		if valueList := findRuleInNodes(nodes[1:], "value_list"); valueList != nil {
			vals := evalValueList(valueList, rowCtx)
			return evalIn(lhs, vals)
		}
		return nil

	case firstTok == "NOT" && len(tokens) >= 2 && strings.ToUpper(tokens[1].Value) == "IN":
		// NOT IN (value_list)
		if valueList := findRuleInNodes(nodes[1:], "value_list"); valueList != nil {
			vals := evalValueList(valueList, rowCtx)
			result := evalIn(lhs, vals)
			if result == nil {
				return nil
			}
			if b, ok := result.(bool); ok {
				return !b
			}
			return nil
		}
		return nil

	case firstTok == "LIKE":
		// LIKE pattern: SQL wildcard matching (% and _)
		if len(nodes) >= 2 {
			pattern := evalExpr(nodes[1], rowCtx)
			return evalLike(lhs, pattern)
		}
		return nil

	case firstTok == "NOT" && len(tokens) >= 2 && strings.ToUpper(tokens[1].Value) == "LIKE":
		// NOT LIKE pattern
		if len(nodes) >= 2 {
			pattern := evalExpr(nodes[1], rowCtx)
			result := evalLike(lhs, pattern)
			if result == nil {
				return nil
			}
			if b, ok := result.(bool); ok {
				return !b
			}
			return nil
		}
		return nil
	}

	return lhs
}

// extractCmpOp extracts the comparison operator string from a cmp_op AST node.
// The cmp_op rule matches one of: =, !=, <, >, <=, >=
// The grammar uses NOT_EQUALS as the token type for both "!=" and "<>".
func extractCmpOp(node *parser.ASTNode) string {
	for _, child := range node.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		// The token value is the actual operator character sequence.
		return tok.Value
	}
	return "="
}

// evalBinaryComparison applies a binary comparison operator to two values.
// Returns bool or nil (if either operand is NULL).
func evalBinaryComparison(lhs interface{}, op string, rhs interface{}) interface{} {
	// Propagate columnNotFound errors up before any other checks.
	// If either side failed to resolve a column, we must propagate the error
	// rather than silently treating it as false/null.
	if _, ok := lhs.(*columnNotFound); ok {
		return lhs // propagate the error up
	}
	if _, ok := rhs.(*columnNotFound); ok {
		return rhs // propagate the error up
	}

	// SQL NULL propagation: any comparison involving NULL is NULL.
	if lhs == nil || rhs == nil {
		return nil
	}

	cmp := compareValues(lhs, rhs)
	switch op {
	case "=":
		return cmp == 0
	case "!=", "<>":
		return cmp != 0
	case "<":
		return cmp < 0
	case ">":
		return cmp > 0
	case "<=":
		return cmp <= 0
	case ">=":
		return cmp >= 0
	}
	return nil
}

// evalBetween implements the BETWEEN predicate.
// "x BETWEEN a AND b" is equivalent to "x >= a AND x <= b".
// Returns nil if any operand is NULL (NULL propagation).
func evalBetween(val, low, high interface{}) interface{} {
	if val == nil || low == nil || high == nil {
		return nil
	}
	geqLow := compareValues(val, low) >= 0
	leqHigh := compareValues(val, high) <= 0
	return geqLow && leqHigh
}

// evalValueList evaluates all expressions in a value_list node.
// value_list = expr { "," expr }
func evalValueList(node *parser.ASTNode, rowCtx map[string]interface{}) []interface{} {
	var vals []interface{}
	collectValueListValues(node, rowCtx, &vals)
	return vals
}

func collectValueListValues(node *parser.ASTNode, rowCtx map[string]interface{}, vals *[]interface{}) {
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == "expr" {
			*vals = append(*vals, evalExpr(childNode, rowCtx))
			continue
		}
		collectValueListValues(childNode, rowCtx, vals)
	}
}

// evalIn implements the IN predicate.
// Returns true if val equals any value in the list, false if no match,
// or nil if val is NULL or any comparison result is NULL and there was no match.
//
// SQL standard: NULL IN (1, 2, 3) = NULL; 1 IN (1, NULL, 2) = TRUE
func evalIn(val interface{}, list []interface{}) interface{} {
	if val == nil {
		return nil
	}
	seenNull := false
	for _, v := range list {
		if v == nil {
			seenNull = true
			continue
		}
		if compareValues(val, v) == 0 {
			return true // found a match
		}
	}
	if seenNull {
		return nil // x IN (NULL, ...) where x != any non-NULL member = NULL
	}
	return false
}

// evalLike implements SQL LIKE pattern matching.
//
// SQL LIKE supports two wildcards:
//   - % matches any sequence of zero or more characters
//   - _ matches exactly one character
//
// Everything else is a literal character match. LIKE is case-sensitive in
// standard SQL (unlike ILIKE which is PostgreSQL-specific).
//
// Implementation: we convert the SQL pattern to a simple state machine rather
// than using regex, which would require escaping many characters.
func evalLike(val, pattern interface{}) interface{} {
	if val == nil || pattern == nil {
		return nil
	}
	s, sOk := val.(string)
	p, pOk := pattern.(string)
	if !sOk || !pOk {
		return nil
	}
	return likeMatch(s, p)
}

// likeMatch is the recursive SQL LIKE pattern matcher.
// This uses a classic dynamic programming / recursive approach.
func likeMatch(s, p string) bool {
	if p == "" {
		return s == ""
	}
	if p[0] == '%' {
		// % matches zero or more characters: try matching zero chars, then one, etc.
		if likeMatch(s, p[1:]) {
			return true // zero chars consumed
		}
		for i := range s {
			if likeMatch(s[i+len(string([]rune(s)[i:])[0:1]):], p[1:]) {
				_ = i
			}
		}
		// Try consuming one character at a time.
		for i := 1; i <= len(s); i++ {
			if likeMatch(s[i:], p[1:]) {
				return true
			}
		}
		return false
	}
	if p[0] == '_' {
		// _ matches exactly one character.
		if len(s) == 0 {
			return false
		}
		return likeMatch(s[1:], p[1:])
	}
	// Literal character: must match exactly.
	if len(s) == 0 || s[0] != p[0] {
		return false
	}
	return likeMatch(s[1:], p[1:])
}

// ─── Arithmetic ──────────────────────────────────────────────────────────────

// evalAdditive handles: additive = multiplicative { ( "+" | "-" ) multiplicative }
//
// Grammar structure in Children (example for "a + b - c"):
//   - multiplicative node (a)
//   - PLUS token
//   - multiplicative node (b)
//   - MINUS token
//   - multiplicative node (c)
//
// NULL propagation: if any operand is NULL, the result is NULL.
func evalAdditive(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	if len(node.Children) == 0 {
		return nil
	}

	// Collect children into alternating value/operator sequence.
	var values []interface{}
	var ops []string

	for _, child := range node.Children {
		switch v := child.(type) {
		case *parser.ASTNode:
			values = append(values, evalExpr(v, rowCtx))
		case lexer.Token:
			if v.Value == "+" || v.Value == "-" {
				ops = append(ops, v.Value)
			}
		}
	}

	if len(values) == 0 {
		return nil
	}

	result := values[0]
	for i, op := range ops {
		if i+1 >= len(values) {
			break
		}
		right := values[i+1]
		result = applyArithmetic(result, op, right)
		if result == nil {
			return nil // NULL propagation
		}
	}
	return result
}

// evalMultiplicative handles: multiplicative = unary { ( STAR | "/" | "%" ) unary }
//
// Division by zero returns NULL (matching PostgreSQL behavior for integer
// division; SQL standard says it should raise an error, but NULL is more
// practical for an embedded engine).
func evalMultiplicative(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	if len(node.Children) == 0 {
		return nil
	}

	var values []interface{}
	var ops []string

	for _, child := range node.Children {
		switch v := child.(type) {
		case *parser.ASTNode:
			values = append(values, evalExpr(v, rowCtx))
		case lexer.Token:
			// STAR is "*" in this context (multiplication, not SELECT *)
			if v.Value == "*" || v.Value == "/" || v.Value == "%" {
				ops = append(ops, v.Value)
			}
		}
	}

	if len(values) == 0 {
		return nil
	}

	result := values[0]
	for i, op := range ops {
		if i+1 >= len(values) {
			break
		}
		right := values[i+1]
		result = applyArithmetic(result, op, right)
		if result == nil {
			return nil
		}
	}
	return result
}

// applyArithmetic performs a binary arithmetic operation on two SQL values.
// Returns nil for NULL propagation or division by zero.
func applyArithmetic(a interface{}, op string, b interface{}) interface{} {
	// Propagate columnNotFound errors.
	if _, ok := a.(*columnNotFound); ok {
		return a
	}
	if _, ok := b.(*columnNotFound); ok {
		return b
	}

	if a == nil || b == nil {
		return nil
	}

	af, aOk := toFloat64(a)
	bf, bOk := toFloat64(b)

	if !aOk || !bOk {
		// String concatenation for "+" with strings (non-standard but convenient).
		if op == "+" {
			as, aStr := a.(string)
			bs, bStr := b.(string)
			if aStr && bStr {
				return as + bs
			}
		}
		return nil
	}

	var result float64
	switch op {
	case "+":
		result = af + bf
	case "-":
		result = af - bf
	case "*":
		result = af * bf
	case "/":
		if bf == 0 {
			return nil // division by zero → NULL
		}
		result = af / bf
	case "%":
		if bf == 0 {
			return nil
		}
		result = math.Mod(af, bf)
	default:
		return nil
	}

	// Preserve int64 type if both operands were integers and result is whole.
	_, aIsInt := a.(int64)
	_, bIsInt := b.(int64)
	if aIsInt && bIsInt && result == math.Trunc(result) && op != "/" {
		return int64(result)
	}
	if result == math.Trunc(result) && op != "/" {
		_, aIsInt32 := a.(int32)
		_, bIsInt32 := b.(int32)
		if (aIsInt || aIsInt32) && (bIsInt || bIsInt32) {
			return int64(result)
		}
	}
	return result
}

// evalUnary handles: unary = "-" unary | primary
//
// Unary minus negates a numeric value. NOT is handled by not_expr.
func evalUnary(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	hasMinus := false
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok && tok.Value == "-" {
			hasMinus = !hasMinus // handle double negation: --x = x
		}
	}

	// Find the inner expression (primary or another unary).
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		val := evalExpr(childNode, rowCtx)
		if !hasMinus {
			return val
		}
		if val == nil {
			return nil
		}
		switch n := val.(type) {
		case int64:
			return -n
		case float64:
			return -n
		}
		return nil
	}
	return nil
}

// ─── Primary ─────────────────────────────────────────────────────────────────

// evalPrimary handles the primary rule:
//
//	primary = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
//	        | function_call | column_ref | "(" expr ")"
//
// Primary is the bottom of the expression hierarchy — it handles literals,
// column references, function calls, and parenthesized subexpressions.
func evalPrimary(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	for _, child := range node.Children {
		switch v := child.(type) {
		case lexer.Token:
			return evalLiteralToken(v)
		case *parser.ASTNode:
			switch v.RuleName {
			case "column_ref":
				return evalColumnRef(v, rowCtx)
			case "function_call":
				// scalar function_call in a non-aggregate context.
				// Aggregate functions are pre-computed by the executor.
				// Here we handle a function_call that the executor placed
				// pre-computed values for (via the rowCtx "_agg_" keys).
				return evalFunctionCallScalar(v, rowCtx)
			case "expr":
				// Parenthesized expression: "(" expr ")"
				return evalExpr(v, rowCtx)
			default:
				return evalExpr(v, rowCtx)
			}
		}
	}
	return nil
}

// evalLiteralToken converts a token to its SQL value.
func evalLiteralToken(tok lexer.Token) interface{} {
	switch {
	case tok.Value == "NULL":
		return nil
	case tok.Value == "TRUE":
		return true
	case tok.Value == "FALSE":
		return false
	case tok.TypeName == "NUMBER" || tok.Type == lexer.TokenNumber:
		return parseNumber(tok.Value)
	case tok.TypeName == "STRING" || tok.Type == lexer.TokenString:
		// String literals are quoted: 'hello'. Strip the single quotes.
		s := tok.Value
		if len(s) >= 2 && s[0] == '\'' && s[len(s)-1] == '\'' {
			return s[1 : len(s)-1]
		}
		return s
	}
	return nil
}

// parseNumber converts a numeric string to int64 or float64.
// We prefer int64 when the value has no decimal point, so that integer
// arithmetic stays integer (matches SQL's typed value system).
func parseNumber(s string) interface{} {
	// Try integer first.
	if !strings.ContainsAny(s, ".eE") {
		if n, err := strconv.ParseInt(s, 10, 64); err == nil {
			return n
		}
	}
	// Fall back to float.
	if f, err := strconv.ParseFloat(s, 64); err == nil {
		return f
	}
	return nil
}

// ─── Column reference ─────────────────────────────────────────────────────────

// evalColumnRef resolves a column reference from the row context.
//
// column_ref = NAME [ "." NAME ]
//
// For single-table queries: "salary" → rowCtx["salary"]
// For multi-table queries: "e.salary" → rowCtx["employees.salary"] (if "e" is
//
//	the alias for "employees"), or rowCtx["e.salary"]
//
// Resolution order:
//  1. Qualified name exactly as written: "dept.name" → rowCtx["dept.name"]
//  2. Unqualified name: "name" → rowCtx["name"]
//  3. Suffix match: "name" → any key ending in ".name" (for JOINs)
func evalColumnRef(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	var parts []string
	for _, child := range node.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if tok.Value != "." {
			parts = append(parts, tok.Value)
		}
	}

	if len(parts) == 0 {
		return nil
	}

	// Build the qualified key: "table.column" or just "column".
	var key string
	if len(parts) == 2 {
		key = parts[0] + "." + parts[1]
	} else {
		key = parts[0]
	}

	// Try exact match first.
	if val, ok := rowCtx[key]; ok {
		return val
	}

	// For unqualified names, try suffix match against "table.column" keys.
	// This allows "SELECT name FROM employees" to resolve "name" to
	// rowCtx["employees.name"] in a joined query.
	if len(parts) == 1 {
		suffix := "." + parts[0]
		matches := 0
		var matchVal interface{}
		for k, v := range rowCtx {
			if strings.HasSuffix(k, suffix) {
				matches++
				matchVal = v
			}
		}
		if matches == 1 {
			return matchVal
		}
		// Multiple matches (ambiguous column): return the first one found.
		// A production engine would raise an error here.
		if matches > 1 {
			return matchVal
		}
	}

	// Column not found: return a sentinel error wrapped as nil.
	// The executor checks for ColumnNotFoundError separately.
	return &columnNotFound{name: key}
}

// columnNotFound is an internal sentinel that signals a missing column.
// It is distinct from nil (SQL NULL) so we can report proper errors.
type columnNotFound struct {
	name string
}

// evalFunctionCallScalar handles scalar (non-aggregate) function calls.
// Currently the engine treats all named functions as aggregates unless they
// are pre-computed values stored in rowCtx. This function primarily handles
// the case where an aggregate result has been pre-stored.
func evalFunctionCallScalar(node *parser.ASTNode, rowCtx map[string]interface{}) interface{} {
	// Extract function name from first token.
	fnName := ""
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

	// Check if the executor pre-stored this aggregate result.
	aggKey := fmt.Sprintf("_agg_%s", fnName)
	if val, ok := rowCtx[aggKey]; ok {
		return val
	}

	return nil
}

// ─── Boolean logic helpers ────────────────────────────────────────────────────

// sqlAnd implements SQL three-valued AND logic.
//
// Truth table (T=true, F=false, N=null):
//
//	T AND T = T    F AND T = F    N AND T = N
//	T AND F = F    F AND F = F    N AND F = F  ← key: F AND N = F (not N)
//	T AND N = N    F AND N = F    N AND N = N
func sqlAnd(a, b interface{}) interface{} {
	// FALSE dominates AND: if either is false, result is false.
	if a == false || b == false {
		return false
	}
	// NULL propagates when combined with TRUE or NULL.
	if a == nil || b == nil {
		return nil
	}
	// Both are truthy (true or non-false, non-nil).
	return true
}

// sqlOr implements SQL three-valued OR logic.
//
// Truth table (T=true, F=false, N=null):
//
//	T OR T = T    F OR T = T    N OR T = T  ← key: T OR N = T (not N)
//	T OR F = T    F OR F = F    N OR F = N
//	T OR N = T    F OR N = N    N OR N = N
func sqlOr(a, b interface{}) interface{} {
	// TRUE dominates OR: if either is true, result is true.
	if a == true || b == true {
		return true
	}
	// NULL propagates when combined with FALSE or NULL.
	if a == nil || b == nil {
		return nil
	}
	// Both are false.
	return false
}

// isTruthy returns whether a SQL value is truthy for WHERE/HAVING filtering.
// In SQL, only TRUE passes the filter — FALSE and NULL both exclude the row.
func isTruthy(v interface{}) bool {
	if v == nil {
		return false // NULL is not truthy
	}
	if b, ok := v.(bool); ok {
		return b
	}
	// Non-boolean non-nil values are truthy (consistent with most SQL engines).
	return true
}

// ─── Utilities ────────────────────────────────────────────────────────────────

// collectRuleChildren returns all child ASTNodes with the given rule name.
// This is used to collect operands in binary operator chains like:
//
//	or_expr = and_expr { "OR" and_expr }
//
// where we need just the and_expr children, skipping "OR" tokens.
func collectRuleChildren(node *parser.ASTNode, ruleName string) []*parser.ASTNode {
	var result []*parser.ASTNode
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == ruleName {
			result = append(result, childNode)
		}
	}
	return result
}

func findRuleInNodes(nodes []*parser.ASTNode, ruleName string) *parser.ASTNode {
	for _, node := range nodes {
		if found := findChildDeep(node, ruleName); found != nil {
			return found
		}
	}
	return nil
}

// findChild searches the direct children of node for an ASTNode with the
// given rule name. Returns nil if not found. This is a depth-1 search.
func findChild(node *parser.ASTNode, ruleName string) *parser.ASTNode {
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == ruleName {
			return childNode
		}
	}
	return nil
}

// findChildDeep searches the entire subtree for the first node with ruleName.
// This is used when we know a node exists somewhere in the tree but don't
// know exactly how many levels deep.
func findChildDeep(node *parser.ASTNode, ruleName string) *parser.ASTNode {
	if node == nil {
		return nil
	}
	if node.RuleName == ruleName {
		return node
	}
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if found := findChildDeep(childNode, ruleName); found != nil {
			return found
		}
	}
	return nil
}

// extractTokenValue finds the first token in node.Children with the given
// value (case-insensitive). Returns "" if not found.
func hasTokenValue(node *parser.ASTNode, value string) bool {
	for _, child := range node.Children {
		tok, ok := child.(lexer.Token)
		if !ok {
			continue
		}
		if strings.EqualFold(tok.Value, value) {
			return true
		}
	}
	return false
}

// hasAggregateInExpr recursively checks if an expression tree contains
// any aggregate function call. This determines whether a query needs
// GROUP BY processing even without an explicit GROUP BY clause.
func hasAggregateInExpr(node *parser.ASTNode) bool {
	if node == nil {
		return false
	}
	if node.RuleName == "function_call" {
		// Extract function name.
		for _, child := range node.Children {
			tok, ok := child.(lexer.Token)
			if !ok {
				continue
			}
			if tok.TypeName == "NAME" || tok.Type == lexer.TokenName {
				if isAggregateFunction(strings.ToUpper(tok.Value)) {
					return true
				}
				break
			}
		}
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			if hasAggregateInExpr(childNode) {
				return true
			}
		}
	}
	return false
}

// isLetter checks if a rune is a letter (for identifier parsing).
func isLetter(r rune) bool {
	return unicode.IsLetter(r)
}

// ensure isLetter is used (it's used by the package, needed to satisfy compiler)
var _ = isLetter
