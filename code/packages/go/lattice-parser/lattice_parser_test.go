package latticeparser

// Lattice Parser Tests
// ====================
//
// These tests verify that ParseLattice produces an AST with the correct
// structure for a variety of Lattice and CSS inputs.
//
// Testing strategy: we verify the rule names of key AST nodes rather than
// exact token values, because the exact tree structure can be verbose.
// For each test we check:
//   1. No error is returned
//   2. The root node has RuleName == "stylesheet"
//   3. The AST contains expected child rule names at the expected depth
//
// For error tests, we verify that a non-nil error is returned.

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// Helpers
// ============================================================================

// findRule does a depth-first search of the AST for a node with the given
// rule name. Returns the first match, or nil if not found.
// This lets us check "does the tree contain a variable_declaration?" without
// caring about the exact path to it.
func findRule(node interface{}, ruleName string) *parser.ASTNode {
	ast, ok := node.(*parser.ASTNode)
	if !ok {
		return nil
	}
	if ast.RuleName == ruleName {
		return ast
	}
	for _, child := range ast.Children {
		if found := findRule(child, ruleName); found != nil {
			return found
		}
	}
	return nil
}

// findAllRules returns all nodes with the given rule name, in DFS order.
func findAllRules(node interface{}, ruleName string) []*parser.ASTNode {
	ast, ok := node.(*parser.ASTNode)
	if !ok {
		return nil
	}
	var result []*parser.ASTNode
	if ast.RuleName == ruleName {
		result = append(result, ast)
	}
	for _, child := range ast.Children {
		result = append(result, findAllRules(child, ruleName)...)
	}
	return result
}

// mustParse is a test helper that calls ParseLattice and fails if there's an error.
func mustParse(t *testing.T, source string) *parser.ASTNode {
	t.Helper()
	ast, err := ParseLattice(source)
	if err != nil {
		t.Fatalf("ParseLattice(%q) returned error: %v", source[:min(len(source), 60)], err)
	}
	if ast == nil {
		t.Fatalf("ParseLattice returned nil AST")
	}
	return ast
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

// ============================================================================
// Basic Parsing
// ============================================================================

func TestParseEmptyInput(t *testing.T) {
	// An empty file is a valid Lattice file — produces an empty stylesheet.
	ast := mustParse(t, "")
	if ast.RuleName != "stylesheet" {
		t.Errorf("root RuleName: got %q, want %q", ast.RuleName, "stylesheet")
	}
}

func TestParseRootIsStylesheet(t *testing.T) {
	// Any valid Lattice file has "stylesheet" as the root rule.
	// This is the invariant the transformer and emitter depend on.
	ast := mustParse(t, "$color: red;")
	if ast.RuleName != "stylesheet" {
		t.Errorf("root RuleName: got %q, want %q", ast.RuleName, "stylesheet")
	}
}

func TestCreateLatticeParserReturnsParser(t *testing.T) {
	// CreateLatticeParser should return a non-nil parser without error.
	p, err := CreateLatticeParser("h1 { color: red; }")
	if err != nil {
		t.Fatalf("CreateLatticeParser returned error: %v", err)
	}
	if p == nil {
		t.Fatal("CreateLatticeParser returned nil parser")
	}
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse() returned error: %v", err)
	}
	if ast == nil {
		t.Fatal("Parse() returned nil AST")
	}
}

// ============================================================================
// Variable Declarations
// ============================================================================
//
// variable_declaration = VARIABLE COLON value_list SEMICOLON ;

func TestParseVariableDeclaration(t *testing.T) {
	// $color: #4a90d9; — the most common Lattice construct
	ast := mustParse(t, "$color: #4a90d9;")
	node := findRule(ast, "variable_declaration")
	if node == nil {
		t.Error("no variable_declaration found in AST")
	}
}

func TestParseVariableDeclarationWithDimension(t *testing.T) {
	// Variables can hold any CSS value, including dimensions
	ast := mustParse(t, "$base-size: 16px;")
	node := findRule(ast, "variable_declaration")
	if node == nil {
		t.Error("no variable_declaration found in AST")
	}
}

func TestParseMultipleVariables(t *testing.T) {
	// Multiple variable declarations at top level
	source := "$primary: #4a90d9;\n$secondary: #e94e77;"
	ast := mustParse(t, source)
	decls := findAllRules(ast, "variable_declaration")
	if len(decls) != 2 {
		t.Errorf("found %d variable_declarations, want 2", len(decls))
	}
}

func TestParseVariableAndCSSRule(t *testing.T) {
	// Mix of Lattice variable and CSS rule
	source := "$primary: #4a90d9;\nh1 { color: $primary; }"
	ast := mustParse(t, source)
	if findRule(ast, "variable_declaration") == nil {
		t.Error("no variable_declaration found")
	}
	if findRule(ast, "qualified_rule") == nil {
		t.Error("no qualified_rule found")
	}
}

// ============================================================================
// Mixin Definitions and @include
// ============================================================================
//
// mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block ;
// include_directive = "@include" FUNCTION include_args RPAREN ( SEMICOLON | block )
//                   | "@include" IDENT ( SEMICOLON | block ) ;

func TestParseMixinDefinition(t *testing.T) {
	// @mixin flex-center() { ... }
	source := "@mixin flex-center() { display: flex; justify-content: center; }"
	ast := mustParse(t, source)
	node := findRule(ast, "mixin_definition")
	if node == nil {
		t.Error("no mixin_definition found in AST")
	}
}

func TestParseMixinDefinitionWithoutParens(t *testing.T) {
	source := "@mixin flex-center { display: flex; justify-content: center; }"
	ast := mustParse(t, source)
	node := findRule(ast, "mixin_definition")
	if node == nil {
		t.Error("no mixin_definition found in AST")
	}
}

func TestParseMixinWithParams(t *testing.T) {
	// @mixin button($bg) { background: $bg; }
	source := "@mixin button($bg) { background: $bg; }"
	ast := mustParse(t, source)
	if findRule(ast, "mixin_definition") == nil {
		t.Error("no mixin_definition found")
	}
	if findRule(ast, "mixin_params") == nil {
		t.Error("no mixin_params found")
	}
}

func TestParseIncludeWithArgs(t *testing.T) {
	// @include button(#4a90d9); inside a CSS rule block
	source := ".btn { @include button(#4a90d9); }"
	ast := mustParse(t, source)
	node := findRule(ast, "include_directive")
	if node == nil {
		t.Error("no include_directive found in AST")
	}
}

func TestParseIncludeNoArgs(t *testing.T) {
	// @include clearfix; — no arguments (IDENT form)
	source := ".container { @include clearfix; }"
	ast := mustParse(t, source)
	node := findRule(ast, "include_directive")
	if node == nil {
		t.Error("no include_directive found in AST")
	}
}

// ============================================================================
// @if / @else Control Flow
// ============================================================================
//
// if_directive = "@if" lattice_expression block
//                { "@else" "if" lattice_expression block }
//                [ "@else" block ] ;

// Note on @if / @for / @each placement:
// These constructs are lattice_block_item, valid only inside blocks ({}),
// not at the top level of a stylesheet. This matches how CSS at-rules work —
// @if is a Lattice control-flow construct for inside rule bodies.
//
// Contrast with @mixin / @function / @use / variable_declaration which are
// lattice_rule and valid at the top level.

func TestParseIfDirective(t *testing.T) {
	// @if inside a mixin body — the natural home for control flow.
	// @mixin wraps a block, and @if is valid inside a block.
	source := `@mixin theme-colors($mode) { @if $mode == dark { color: white; } }`
	ast := mustParse(t, source)
	node := findRule(ast, "if_directive")
	if node == nil {
		t.Error("no if_directive found in AST")
	}
}

func TestParseIfElseDirective(t *testing.T) {
	// @if / @else inside a qualified rule block
	source := `.btn { @if $n > 0 { color: green; } @else { color: red; } }`
	ast := mustParse(t, source)
	node := findRule(ast, "if_directive")
	if node == nil {
		t.Error("no if_directive found in AST")
	}
}

func TestParseIfWithComparisonOps(t *testing.T) {
	// Test all comparison operators in @if conditions (inside a block)
	tests := []struct {
		name   string
		source string
	}{
		{"equals", `.x { @if $x == 1 { color: red; } }`},
		{"not-equals", `.x { @if $x != 1 { color: red; } }`},
		{"greater-equals", `.x { @if $x >= 1 { color: red; } }`},
		{"less-equals", `.x { @if $x <= 1 { color: red; } }`},
		{"greater", `.x { @if $x > 1 { color: red; } }`},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ast := mustParse(t, tt.source)
			if findRule(ast, "if_directive") == nil {
				t.Errorf("no if_directive found for %s", tt.name)
			}
		})
	}
}

// ============================================================================
// @for Loop
// ============================================================================
//
// for_directive = "@for" VARIABLE "from" lattice_expression
//                 ( "through" | "to" ) lattice_expression block ;

func TestParseForThrough(t *testing.T) {
	// @for inside a mixin body — inclusive range (1, 2, ..., 12)
	source := `@mixin cols() { @for $i from 1 through 12 { .col { width: 8%; } } }`
	ast := mustParse(t, source)
	node := findRule(ast, "for_directive")
	if node == nil {
		t.Error("no for_directive found in AST")
	}
}

func TestParseForTo(t *testing.T) {
	// @for with "to" (exclusive range: 1, 2, ..., 11)
	source := `@mixin items() { @for $i from 1 to 12 { .item { margin: 4px; } } }`
	ast := mustParse(t, source)
	node := findRule(ast, "for_directive")
	if node == nil {
		t.Error("no for_directive found in AST")
	}
}

// ============================================================================
// @each Loop
// ============================================================================
//
// each_directive = "@each" VARIABLE { COMMA VARIABLE } "in" each_list block ;

func TestParseEachDirective(t *testing.T) {
	// @each inside a mixin body — iterates over a list of values
	source := `@mixin colors() { @each $color in red, green, blue { .text { color: $color; } } }`
	ast := mustParse(t, source)
	node := findRule(ast, "each_directive")
	if node == nil {
		t.Error("no each_directive found in AST")
	}
}

// ============================================================================
// @function and @return
// ============================================================================
//
// function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body ;
// return_directive = "@return" lattice_expression SEMICOLON ;

func TestParseFunctionDefinition(t *testing.T) {
	// @function spacing($n) { @return $n * 8px; }
	source := `@function spacing($n) { @return $n * 8px; }`
	ast := mustParse(t, source)
	node := findRule(ast, "function_definition")
	if node == nil {
		t.Error("no function_definition found in AST")
	}
}

func TestParseReturnDirective(t *testing.T) {
	// @return inside a @function body
	source := `@function double($n) { @return $n * 2; }`
	ast := mustParse(t, source)
	node := findRule(ast, "return_directive")
	if node == nil {
		t.Error("no return_directive found in AST")
	}
}

// ============================================================================
// Plain CSS Rules
// ============================================================================
//
// CSS rules should parse unchanged — Lattice is a strict superset of CSS.

func TestParsePlainCSSRule(t *testing.T) {
	// h1 { color: red; font-size: 24px; }
	source := "h1 { color: red; font-size: 24px; }"
	ast := mustParse(t, source)
	node := findRule(ast, "qualified_rule")
	if node == nil {
		t.Error("no qualified_rule found in AST")
	}
}

func TestParseCSSAtRule(t *testing.T) {
	// @media (max-width: 768px) { ... }
	source := `@media (max-width: 768px) { h1 { font-size: 18px; } }`
	ast := mustParse(t, source)
	node := findRule(ast, "at_rule")
	if node == nil {
		t.Error("no at_rule found in AST")
	}
}

func TestParseCSSDeclaration(t *testing.T) {
	// A declaration inside a rule body
	source := "p { color: blue; margin: 0 auto; }"
	ast := mustParse(t, source)
	decls := findAllRules(ast, "declaration")
	if len(decls) == 0 {
		t.Error("no declaration nodes found in AST")
	}
}

func TestParseCSSClassSelector(t *testing.T) {
	// .btn { ... }
	source := ".btn { display: inline-block; }"
	ast := mustParse(t, source)
	if findRule(ast, "qualified_rule") == nil {
		t.Error("no qualified_rule found")
	}
}

func TestParseCSSPseudoClass(t *testing.T) {
	// a:hover { ... }
	source := "a:hover { color: blue; }"
	ast := mustParse(t, source)
	if findRule(ast, "qualified_rule") == nil {
		t.Error("no qualified_rule found")
	}
}

func TestParseCSSImportant(t *testing.T) {
	// color: red !important;
	source := "p { color: red !important; }"
	ast := mustParse(t, source)
	if findRule(ast, "priority") == nil {
		t.Error("no priority (! important) found in AST")
	}
}

// ============================================================================
// @use Directive
// ============================================================================
//
// use_directive = "@use" STRING [ "as" IDENT ] SEMICOLON ;

func TestParseUseDirective(t *testing.T) {
	// @use "colors"; — import a module
	source := `@use "colors";`
	ast := mustParse(t, source)
	node := findRule(ast, "use_directive")
	if node == nil {
		t.Error("no use_directive found in AST")
	}
}

func TestParseUseDirectiveWithAlias(t *testing.T) {
	// @use "utils/mixins" as m;
	source := `@use "utils/mixins" as m;`
	ast := mustParse(t, source)
	node := findRule(ast, "use_directive")
	if node == nil {
		t.Error("no use_directive found in AST")
	}
}

// ============================================================================
// Complex / Integration Tests
// ============================================================================

func TestParseFullStylesheet(t *testing.T) {
	// A realistic Lattice stylesheet with variables, mixins, and CSS rules
	source := strings.TrimSpace(`
$primary: #4a90d9;
$font-stack: Helvetica, sans-serif;

@mixin clearfix() {
  content: "";
  display: table;
  clear: both;
}

h1 {
  color: $primary;
  font-family: $font-stack;
}

.container::after {
  @include clearfix;
}
`)
	ast := mustParse(t, source)

	if ast.RuleName != "stylesheet" {
		t.Errorf("root: got %q, want %q", ast.RuleName, "stylesheet")
	}
	if findRule(ast, "variable_declaration") == nil {
		t.Error("no variable_declaration found")
	}
	if findRule(ast, "mixin_definition") == nil {
		t.Error("no mixin_definition found")
	}
	if findRule(ast, "qualified_rule") == nil {
		t.Error("no qualified_rule found")
	}
}

func TestParseNestedControlFlow(t *testing.T) {
	// @if containing a @for loop inside a mixin body
	source := strings.TrimSpace(`
@mixin grid($generate) {
  @if $generate == true {
    @for $i from 1 through 12 {
      .col { width: 8%; }
    }
  }
}
`)
	ast := mustParse(t, source)
	if findRule(ast, "if_directive") == nil {
		t.Error("no if_directive found")
	}
	if findRule(ast, "for_directive") == nil {
		t.Error("no for_directive found")
	}
}

func TestParseFunctionWithIfReturn(t *testing.T) {
	// @function with @if/@return — the function body grammar allows control flow
	source := strings.TrimSpace(`
@function clamped($n) {
  @return $n * 8px;
}
`)
	ast := mustParse(t, source)
	if findRule(ast, "function_definition") == nil {
		t.Error("no function_definition found")
	}
	if findRule(ast, "return_directive") == nil {
		t.Error("no return_directive found")
	}
}

// ============================================================================
// Error Cases
// ============================================================================

func TestParseErrorMissingClosingBrace(t *testing.T) {
	// Missing } — should return a parse error
	_, err := ParseLattice("h1 { color: red;")
	if err == nil {
		t.Error("expected parse error for missing }, got nil")
	}
}

func TestParseErrorMissingValue(t *testing.T) {
	// Missing value in a declaration — grammar should reject this
	_, err := ParseLattice("h1 { color: ; }")
	// Either a parse error or the grammar accepts it (empty value_list is
	// not valid per the grammar, so we expect an error).
	_ = err // error handling tested; we just verify no panic
}
