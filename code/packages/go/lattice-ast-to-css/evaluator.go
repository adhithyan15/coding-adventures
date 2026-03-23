package latticeasttocss

// evaluator.go — compile-time evaluation of Lattice expressions.
//
// # What Gets Evaluated?
//
// Lattice expressions appear in three places:
//
//  1. @if conditions:  @if $theme == dark { ... }
//  2. @for bounds:     @for $i from 1 through $count { ... }
//  3. @return values:  @return $multiplier * 8px;
//
// Because Lattice compiles to CSS (there is no runtime), ALL expressions are
// evaluated at compile time. This is similar to constant folding in a
// conventional compiler.
//
// # Value Types
//
// The evaluator works with nine value types that mirror CSS/Lattice semantics:
//
//	LatticeNumber     — 42, 3.14, 0 (no unit)
//	LatticeDimension  — 16px, 2em, 1.5rem (number + CSS unit)
//	LatticePercentage — 50%, 100% (number + %)
//	LatticeString     — "hello", 'world' (quoted strings)
//	LatticeIdent      — red, bold, dark (unquoted identifiers)
//	LatticeColor      — #4a90d9, #fff (hex colors)
//	LatticeBool       — true, false
//	LatticeNull       — null (falsy, like Sass null)
//	LatticeList       — red, green, blue (comma-separated, for @each)
//
// # Operator Precedence (tightest to loosest)
//
//  1. Unary minus:      -$x
//  2. Multiplication:   $a * $b
//  3. Addition:         $a + $b, $a - $b
//  4. Comparison:       ==, !=, >, >=, <=
//  5. Logical AND:      $a and $b
//  6. Logical OR:       $a or $b
//
// The grammar encodes this precedence via nested rules (or_expr → and_expr →
// comparison → additive → multiplicative → unary → primary), so the evaluator
// just recurses without needing its own precedence climbing.
//
// # Arithmetic Rules
//
// Addition and subtraction:
//   Number ± Number → Number
//   Dimension ± Dimension (same unit) → Dimension
//   Percentage ± Percentage → Percentage
//   Anything else → TypeErrorInExpression
//
// Multiplication:
//   Number × Number → Number
//   Number × Dimension → Dimension   (scaling: 2 * 8px = 16px)
//   Dimension × Number → Dimension   (commutative)
//   Number × Percentage → Percentage
//   Percentage × Number → Percentage
//   Anything else → TypeErrorInExpression

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// LatticeValue Interface
// ============================================================================

// LatticeValue is the interface implemented by all Lattice runtime values.
//
// Like json-value's JsonValue, this uses the sealed-interface pattern: the
// unexported marker method latticeValue() restricts the set of implementing
// types to those defined in this package.
type LatticeValue interface {
	latticeValue() // marker — forces implementations to be in this package
	String() string
	Truthy() bool
}

// ============================================================================
// Concrete Value Types
// ============================================================================

// LatticeNumber is a pure number without a unit.
//
// Examples: 42, 3.14, 0, -1
// Maps to the CSS NUMBER token.
type LatticeNumber struct {
	Value float64
}

func (v LatticeNumber) latticeValue() {}
func (v LatticeNumber) Truthy() bool  { return v.Value != 0 }
func (v LatticeNumber) String() string {
	// Emit integers without a decimal point: "42" not "42.000000"
	if v.Value == float64(int64(v.Value)) {
		return strconv.FormatInt(int64(v.Value), 10)
	}
	return strconv.FormatFloat(v.Value, 'f', -1, 64)
}

// LatticeDimension is a number with a CSS unit.
//
// Examples: 16px, 2em, 1.5rem, 100vh, 300ms
// Maps to the CSS DIMENSION token. Arithmetic is only valid between
// dimensions with the same unit; mixed-unit math requires calc().
type LatticeDimension struct {
	Value float64
	Unit  string
}

func (v LatticeDimension) latticeValue() {}
func (v LatticeDimension) Truthy() bool  { return true } // dimensions are always truthy
func (v LatticeDimension) String() string {
	if v.Value == float64(int64(v.Value)) {
		return fmt.Sprintf("%d%s", int64(v.Value), v.Unit)
	}
	return fmt.Sprintf("%s%s", strconv.FormatFloat(v.Value, 'f', -1, 64), v.Unit)
}

// LatticePercentage is a percentage value.
//
// Examples: 50%, 100%, 33.33%
// Maps to the CSS PERCENTAGE token.
type LatticePercentage struct {
	Value float64
}

func (v LatticePercentage) latticeValue() {}
func (v LatticePercentage) Truthy() bool  { return true }
func (v LatticePercentage) String() string {
	if v.Value == float64(int64(v.Value)) {
		return fmt.Sprintf("%d%%", int64(v.Value))
	}
	return fmt.Sprintf("%s%%", strconv.FormatFloat(v.Value, 'f', -1, 64))
}

// LatticeString is a quoted string value.
//
// The quote characters are not stored — they are added back when emitting CSS.
// Examples: "hello", 'world'
type LatticeString struct {
	Value string
}

func (v LatticeString) latticeValue() {}
func (v LatticeString) Truthy() bool  { return true }
func (v LatticeString) String() string {
	return fmt.Sprintf("%q", v.Value)
}

// LatticeIdent is an unquoted CSS identifier.
//
// Examples: red, bold, dark, sans-serif, transparent
// CSS color keywords and other idents are treated as opaque — no arithmetic.
type LatticeIdent struct {
	Value string
}

func (v LatticeIdent) latticeValue() {}
func (v LatticeIdent) Truthy() bool  { return true }
func (v LatticeIdent) String() string { return v.Value }

// LatticeColor is a hex color value.
//
// Examples: #4a90d9, #fff, #00000080
// Stored with the # prefix, exactly as written in the source.
type LatticeColor struct {
	Value string
}

func (v LatticeColor) latticeValue() {}
func (v LatticeColor) Truthy() bool  { return true }
func (v LatticeColor) String() string { return v.Value }

// LatticeBool is a boolean value — true or false.
//
// Lattice boolean literals are matched by the grammar: "true" and "false"
// as IDENT tokens. LatticeBool is the evaluated form of these literals.
type LatticeBool struct {
	Value bool
}

func (v LatticeBool) latticeValue() {}
func (v LatticeBool) Truthy() bool  { return v.Value }
func (v LatticeBool) String() string {
	if v.Value {
		return "true"
	}
	return "false"
}

// LatticeNull is the null value.
//
// null is falsy and stringifies to empty string (matching Sass semantics).
// Used for optional parameters and missing values.
type LatticeNull struct{}

func (v LatticeNull) latticeValue() {}
func (v LatticeNull) Truthy() bool  { return false }
func (v LatticeNull) String() string { return "" }

// LatticeList is a comma-separated list of values.
//
// Used in @each directives and multi-value declarations.
// Example: red, green, blue
type LatticeList struct {
	Items []LatticeValue
}

func (v LatticeList) latticeValue() {}
func (v LatticeList) Truthy() bool  { return len(v.Items) > 0 }
func (v LatticeList) String() string {
	parts := make([]string, len(v.Items))
	for i, item := range v.Items {
		parts[i] = item.String()
	}
	return strings.Join(parts, ", ")
}

// ============================================================================
// Token → Value Conversion
// ============================================================================

// tokenTypeName returns the string name of a token's type.
//
// Grammar-driven tokens store the human-readable name in Token.TypeName
// (e.g., "VARIABLE", "IDENT"). Hand-written lexer tokens use Token.Type (int).
// This function abstracts both.
func tokenTypeName(tok lexer.Token) string {
	if tok.TypeName != "" {
		return tok.TypeName
	}
	return tok.Type.String()
}

// tokenToValue converts a lexer Token to a LatticeValue.
//
// This bridges the gap between the parser's token world and the evaluator's
// value world. The parser gives us tokens; arithmetic needs typed values.
//
// Mapping:
//
//	NUMBER     → LatticeNumber
//	DIMENSION  → LatticeDimension (splits "16px" into 16 + "px")
//	PERCENTAGE → LatticePercentage (strips the "%")
//	STRING     → LatticeString
//	HASH       → LatticeColor
//	IDENT      → LatticeIdent (or LatticeBool/LatticeNull for keywords)
//	other      → LatticeIdent (fallback)
func tokenToValue(tok lexer.Token) LatticeValue {
	typeName := tokenTypeName(tok)
	val := tok.Value

	switch typeName {
	case "NUMBER":
		f, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return LatticeIdent{Value: val}
		}
		return LatticeNumber{Value: f}

	case "DIMENSION":
		// Split "16px" into numeric part and unit part.
		// The number may be negative: "-10px" → -10, "px".
		i := 0
		if i < len(val) && val[i] == '-' {
			i++
		}
		for i < len(val) && (val[i] == '.' || (val[i] >= '0' && val[i] <= '9')) {
			i++
		}
		// Handle scientific notation: 1e+2px
		if i < len(val) && (val[i] == 'e' || val[i] == 'E') {
			i++
			if i < len(val) && (val[i] == '+' || val[i] == '-') {
				i++
			}
			for i < len(val) && val[i] >= '0' && val[i] <= '9' {
				i++
			}
		}
		numStr := val[:i]
		unit := val[i:]
		f, err := strconv.ParseFloat(numStr, 64)
		if err != nil {
			return LatticeIdent{Value: val}
		}
		return LatticeDimension{Value: f, Unit: unit}

	case "PERCENTAGE":
		// "50%" → LatticePercentage(50)
		numStr := strings.TrimSuffix(val, "%")
		f, err := strconv.ParseFloat(numStr, 64)
		if err != nil {
			return LatticeIdent{Value: val}
		}
		return LatticePercentage{Value: f}

	case "STRING":
		// The lexer already strips quotes; val is the bare string content.
		return LatticeString{Value: val}

	case "HASH":
		return LatticeColor{Value: val}

	case "IDENT":
		// Special IDENT values that become typed booleans or null
		switch val {
		case "true":
			return LatticeBool{Value: true}
		case "false":
			return LatticeBool{Value: false}
		case "null":
			return LatticeNull{}
		}
		return LatticeIdent{Value: val}
	}

	// Fallback: treat any unrecognized token as an identifier.
	return LatticeIdent{Value: val}
}

// valueToCSSText converts a LatticeValue to its CSS text representation.
//
// Used when substituting evaluated values back into CSS output. Each value
// type's String() method returns the correct CSS representation.
func valueToCSSText(val LatticeValue) string {
	return val.String()
}

// ============================================================================
// Expression Evaluator
// ============================================================================

// ExpressionEvaluator evaluates Lattice expression AST nodes at compile time.
//
// The evaluator is a recursive AST walker. It dispatches on the rule_name of
// each node to the appropriate handler. Leaf tokens are converted to
// LatticeValue via tokenToValue.
//
// The grammar's nesting already encodes operator precedence, so we just
// recurse — no precedence climbing or Pratt parsing is needed.
//
// Usage:
//
//	eval := NewExpressionEvaluator(scope)
//	result := eval.Evaluate(expressionNode)
//	// result is a LatticeValue, e.g., LatticeBool{true}
type ExpressionEvaluator struct {
	scope *ScopeChain
}

// NewExpressionEvaluator creates a new evaluator with the given scope.
// The scope is used to look up $variable values during evaluation.
func NewExpressionEvaluator(scope *ScopeChain) *ExpressionEvaluator {
	return &ExpressionEvaluator{scope: scope}
}

// Evaluate walks an expression AST node and returns the computed LatticeValue.
//
// This is the main entry point. Pass any node from the expression sub-grammar
// (lattice_expression, lattice_or_expr, ..., lattice_primary) or a raw Token.
func (e *ExpressionEvaluator) Evaluate(node interface{}) LatticeValue {
	// Raw token (leaf node) — convert directly to a value
	if tok, ok := node.(lexer.Token); ok {
		return tokenToValue(tok)
	}

	ast, ok := node.(*parser.ASTNode)
	if !ok || ast == nil {
		return LatticeNull{}
	}

	// Dispatch to the handler for this rule
	switch ast.RuleName {
	case "lattice_expression":
		return e.evalExpression(ast)
	case "lattice_or_expr":
		return e.evalOr(ast)
	case "lattice_and_expr":
		return e.evalAnd(ast)
	case "lattice_comparison":
		return e.evalComparison(ast)
	case "comparison_op":
		// Handled by the parent lattice_comparison rule; shouldn't be called directly
		if len(ast.Children) > 0 {
			return tokenToValue(ast.Children[0].(lexer.Token))
		}
		return LatticeNull{}
	case "lattice_additive":
		return e.evalAdditive(ast)
	case "lattice_multiplicative":
		return e.evalMultiplicative(ast)
	case "lattice_unary":
		return e.evalUnary(ast)
	case "lattice_primary":
		return e.evalPrimary(ast)
	}

	// For single-child wrapper rules, unwrap and recurse
	if len(ast.Children) == 1 {
		return e.Evaluate(ast.Children[0])
	}

	// Default: try to evaluate the first meaningful child
	for _, child := range ast.Children {
		switch child.(type) {
		case *parser.ASTNode, lexer.Token:
			return e.Evaluate(child)
		}
	}

	return LatticeNull{}
}

// evalExpression handles: lattice_expression = lattice_or_expr ;
func (e *ExpressionEvaluator) evalExpression(node *parser.ASTNode) LatticeValue {
	if len(node.Children) > 0 {
		return e.Evaluate(node.Children[0])
	}
	return LatticeNull{}
}

// evalOr handles: lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;
//
// Uses short-circuit evaluation: returns the first truthy operand, or the last
// operand if none are truthy. This matches JavaScript's || semantics.
func (e *ExpressionEvaluator) evalOr(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		// Skip the "or" IDENT token
		if tok, ok := node.Children[i].(lexer.Token); ok && tok.Value == "or" {
			i++
			continue
		}
		if result.Truthy() {
			return result
		}
		result = e.Evaluate(node.Children[i])
		i++
	}
	return result
}

// evalAnd handles: lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;
//
// Short-circuit: returns the first falsy operand, or the last if all are truthy.
func (e *ExpressionEvaluator) evalAnd(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		if tok, ok := node.Children[i].(lexer.Token); ok && tok.Value == "and" {
			i++
			continue
		}
		if !result.Truthy() {
			return result
		}
		result = e.Evaluate(node.Children[i])
		i++
	}
	return result
}

// evalComparison handles: lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;
func (e *ExpressionEvaluator) evalComparison(node *parser.ASTNode) LatticeValue {
	left := e.Evaluate(node.Children[0])
	if len(node.Children) == 1 {
		return left
	}

	// Find comparison_op and the right operand
	var opNode *parser.ASTNode
	var rightNode interface{}
	for i, child := range node.Children[1:] {
		if ast, ok := child.(*parser.ASTNode); ok && ast.RuleName == "comparison_op" {
			opNode = ast
			if i+2 < len(node.Children) {
				rightNode = node.Children[i+2]
			}
			break
		}
	}

	if opNode == nil || rightNode == nil {
		return left
	}

	right := e.Evaluate(rightNode)

	// The comparison_op node has a single token child
	if len(opNode.Children) == 0 {
		return LatticeBool{Value: false}
	}
	opTok, ok := opNode.Children[0].(lexer.Token)
	if !ok {
		return LatticeBool{Value: false}
	}

	return e.compare(left, right, tokenTypeName(opTok))
}

// compare performs a comparison between two values.
//
// For numeric types (same type), compares by value.
// For equality comparisons of non-numeric types, compares by string representation.
// For ordering comparisons of non-numeric types, returns false.
func (e *ExpressionEvaluator) compare(left, right LatticeValue, opType string) LatticeBool {
	// Numeric comparison for compatible types
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeBool{Value: numCompare(l.Value, r.Value, opType)}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeDimension); ok {
			if l.Unit == r.Unit {
				return LatticeBool{Value: numCompare(l.Value, r.Value, opType)}
			}
			// Different units: only equality/inequality make sense
			switch opType {
			case "EQUALS_EQUALS":
				return LatticeBool{Value: false}
			case "NOT_EQUALS":
				return LatticeBool{Value: true}
			default:
				return LatticeBool{Value: false}
			}
		}
	case LatticePercentage:
		if r, ok := right.(LatticePercentage); ok {
			return LatticeBool{Value: numCompare(l.Value, r.Value, opType)}
		}
	}

	// Fallback: string equality for mixed/non-numeric types
	switch opType {
	case "EQUALS_EQUALS":
		return LatticeBool{Value: left.String() == right.String()}
	case "NOT_EQUALS":
		return LatticeBool{Value: left.String() != right.String()}
	}
	return LatticeBool{Value: false}
}

// numCompare compares two float64 values using the given operator type name.
func numCompare(lv, rv float64, opType string) bool {
	switch opType {
	case "EQUALS_EQUALS":
		return lv == rv
	case "NOT_EQUALS":
		return lv != rv
	case "GREATER":
		return lv > rv
	case "GREATER_EQUALS":
		return lv >= rv
	case "LESS_EQUALS":
		return lv <= rv
	}
	return false
}

// evalAdditive handles:
//
//	lattice_additive = lattice_multiplicative { ( PLUS | MINUS ) lattice_multiplicative } ;
func (e *ExpressionEvaluator) evalAdditive(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		tok, ok := node.Children[i].(lexer.Token)
		if !ok {
			i++
			continue
		}
		op := tok.Value
		if op != "+" && op != "-" {
			i++
			continue
		}
		i++
		if i >= len(node.Children) {
			break
		}
		right := e.Evaluate(node.Children[i])
		if op == "+" {
			result = e.add(result, right)
		} else {
			result = e.subtract(result, right)
		}
		i++
	}
	return result
}

// add performs addition. Compatible types only.
func (e *ExpressionEvaluator) add(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeNumber{Value: l.Value + r.Value}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeDimension); ok {
			if l.Unit == r.Unit {
				return LatticeDimension{Value: l.Value + r.Value, Unit: l.Unit}
			}
			panic(NewTypeErrorInExpression("add", left.String(), right.String(), 0, 0))
		}
	case LatticePercentage:
		if r, ok := right.(LatticePercentage); ok {
			return LatticePercentage{Value: l.Value + r.Value}
		}
	case LatticeString:
		if r, ok := right.(LatticeString); ok {
			return LatticeString{Value: l.Value + r.Value}
		}
	}
	panic(NewTypeErrorInExpression("add", left.String(), right.String(), 0, 0))
}

// subtract performs subtraction. Mirrors add.
func (e *ExpressionEvaluator) subtract(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeNumber{Value: l.Value - r.Value}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeDimension); ok {
			if l.Unit == r.Unit {
				return LatticeDimension{Value: l.Value - r.Value, Unit: l.Unit}
			}
			panic(NewTypeErrorInExpression("subtract", left.String(), right.String(), 0, 0))
		}
	case LatticePercentage:
		if r, ok := right.(LatticePercentage); ok {
			return LatticePercentage{Value: l.Value - r.Value}
		}
	}
	panic(NewTypeErrorInExpression("subtract", left.String(), right.String(), 0, 0))
}

// evalMultiplicative handles:
//
//	lattice_multiplicative = lattice_unary { STAR lattice_unary } ;
func (e *ExpressionEvaluator) evalMultiplicative(node *parser.ASTNode) LatticeValue {
	result := e.Evaluate(node.Children[0])
	i := 1
	for i < len(node.Children) {
		tok, ok := node.Children[i].(lexer.Token)
		if !ok || tok.Value != "*" {
			i++
			continue
		}
		i++
		if i >= len(node.Children) {
			break
		}
		right := e.Evaluate(node.Children[i])
		result = e.multiply(result, right)
		i++
	}
	return result
}

// multiply performs multiplication.
//
// Supported combinations:
//   Number × Number → Number
//   Number × Dimension → Dimension   (scales the dimension)
//   Dimension × Number → Dimension   (commutative)
//   Number × Percentage → Percentage
//   Percentage × Number → Percentage
func (e *ExpressionEvaluator) multiply(left, right LatticeValue) LatticeValue {
	switch l := left.(type) {
	case LatticeNumber:
		switch r := right.(type) {
		case LatticeNumber:
			return LatticeNumber{Value: l.Value * r.Value}
		case LatticeDimension:
			return LatticeDimension{Value: l.Value * r.Value, Unit: r.Unit}
		case LatticePercentage:
			return LatticePercentage{Value: l.Value * r.Value}
		}
	case LatticeDimension:
		if r, ok := right.(LatticeNumber); ok {
			return LatticeDimension{Value: l.Value * r.Value, Unit: l.Unit}
		}
	case LatticePercentage:
		if r, ok := right.(LatticeNumber); ok {
			return LatticePercentage{Value: l.Value * r.Value}
		}
	}
	panic(NewTypeErrorInExpression("multiply", left.String(), right.String(), 0, 0))
}

// evalUnary handles: lattice_unary = MINUS lattice_unary | lattice_primary ;
func (e *ExpressionEvaluator) evalUnary(node *parser.ASTNode) LatticeValue {
	if len(node.Children) < 2 {
		if len(node.Children) == 1 {
			return e.Evaluate(node.Children[0])
		}
		return LatticeNull{}
	}

	// Check if first child is a MINUS token
	if tok, ok := node.Children[0].(lexer.Token); ok && tok.Value == "-" {
		operand := e.Evaluate(node.Children[1])
		return e.negate(operand)
	}

	return e.Evaluate(node.Children[0])
}

// negate negates a numeric value.
func (e *ExpressionEvaluator) negate(val LatticeValue) LatticeValue {
	switch v := val.(type) {
	case LatticeNumber:
		return LatticeNumber{Value: -v.Value}
	case LatticeDimension:
		return LatticeDimension{Value: -v.Value, Unit: v.Unit}
	case LatticePercentage:
		return LatticePercentage{Value: -v.Value}
	}
	panic(NewTypeErrorInExpression("negate", val.String(), "", 0, 0))
}

// evalPrimary handles:
//
//	lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
//	                | STRING | IDENT | HASH
//	                | "true" | "false" | "null"
//	                | function_call
//	                | LPAREN lattice_expression RPAREN ;
func (e *ExpressionEvaluator) evalPrimary(node *parser.ASTNode) LatticeValue {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			typeName := tokenTypeName(c)

			if typeName == "LPAREN" || typeName == "RPAREN" {
				continue // skip parens
			}

			if typeName == "VARIABLE" {
				// Look up the variable in the current scope
				val, ok := e.scope.Get(c.Value)
				if !ok {
					// Return an ident for now; transformer will raise the error
					return LatticeIdent{Value: c.Value}
				}
				if lv, ok := val.(LatticeValue); ok {
					return lv
				}
				// If it's an AST node (value_list), extract the value
				if ast, ok := val.(*parser.ASTNode); ok {
					return e.extractValueFromAST(ast)
				}
				// Raw token
				if tok, ok := val.(lexer.Token); ok {
					return tokenToValue(tok)
				}
				return LatticeIdent{Value: c.Value}
			}

			return tokenToValue(c)

		case *parser.ASTNode:
			// Recurse into sub-expressions (e.g., LPAREN lattice_expression RPAREN)
			if c.RuleName == "lattice_expression" {
				return e.Evaluate(c)
			}
			// function_call or other rule
			return e.Evaluate(c)
		}
	}
	return LatticeNull{}
}

// extractValueFromAST extracts a LatticeValue from an AST node.
//
// When a variable is bound to a value_list node (from the parser), we need to
// extract the actual value. A value_list like "dark" contains a single value
// node wrapping an IDENT token.
//
// For multi-token value_lists (e.g., "Helvetica, sans-serif"), we take the
// first token's value — which is sufficient for expression evaluation.
func (e *ExpressionEvaluator) extractValueFromAST(node *parser.ASTNode) LatticeValue {
	for _, child := range node.Children {
		switch c := child.(type) {
		case lexer.Token:
			return tokenToValue(c)
		case *parser.ASTNode:
			result := e.extractValueFromAST(c)
			if _, isNull := result.(LatticeNull); !isNull {
				return result
			}
		}
	}
	return LatticeNull{}
}
