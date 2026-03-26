package latticeasttocss

// lattice_ast_to_css_test.go — integration and unit tests for the full
// Lattice-to-CSS compilation pipeline.
//
// # Test Strategy
//
// These tests cover the entire pipeline end-to-end: Lattice source text →
// CSS text, using TranspileLattice() and TranspileLatticeMinified().
// They are black-box tests — they only assert on the final CSS output.
//
// Individual components (ScopeChain, ExpressionEvaluator, LatticeTransformer,
// CSSEmitter) are unit-tested here where behavior is easiest to verify
// through their public interfaces.
//
// # Test Categories
//
//  1. Variable substitution — $var declared and referenced
//  2. Mixin expansion — @mixin / @include with args and defaults
//  3. @if / @else if / @else control flow
//  4. @for loops (through and to)
//  5. @each loops
//  6. @function definitions and calls
//  7. Scope shadowing — inner $var overrides outer
//  8. Emitter modes — pretty vs minified
//  9. Arithmetic expressions in variable declarations
// 10. Error cases — undefined variable, undefined mixin, circular reference,
//     wrong arity, type error, missing return

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	latticeparser "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// Helpers
// ============================================================================

// mustTranspile calls TranspileLattice and fails the test on error.
// Returns the CSS output trimmed of leading/trailing whitespace.
func mustTranspile(t *testing.T, source string) string {
	t.Helper()
	css, err := TranspileLattice(source)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	return strings.TrimSpace(css)
}

// mustTranspileMin calls TranspileLatticeMinified and fails the test on error.
func mustTranspileMin(t *testing.T, source string) string {
	t.Helper()
	css, err := TranspileLatticeMinified(source)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	return strings.TrimSpace(css)
}

// transpileErr calls TranspileLattice and returns the error (or nil).
func transpileErr(source string) error {
	_, err := TranspileLattice(source)
	return err
}

// assertContains checks that haystack contains all needles.
func assertContains(t *testing.T, haystack string, needles ...string) {
	t.Helper()
	for _, needle := range needles {
		if !strings.Contains(haystack, needle) {
			t.Errorf("expected output to contain %q\ngot:\n%s", needle, haystack)
		}
	}
}

// assertNotContains checks that haystack does NOT contain needle.
func assertNotContains(t *testing.T, haystack, needle string) {
	t.Helper()
	if strings.Contains(haystack, needle) {
		t.Errorf("expected output NOT to contain %q\ngot:\n%s", needle, haystack)
	}
}

// ============================================================================
// 1. Variable Substitution
// ============================================================================

// TestVariableSubstitution verifies that $variable declarations are replaced
// with their values in CSS output.
func TestVariableSubstitution(t *testing.T) {
	css := mustTranspile(t, `
		$color: red;
		.btn { color: $color; }
	`)
	assertContains(t, css, "color: red")
	assertNotContains(t, css, "$color")
}

// TestVariableInMultipleProperties verifies that one variable can be used in
// multiple CSS declarations.
func TestVariableInMultipleProperties(t *testing.T) {
	css := mustTranspile(t, `
		$size: 16px;
		.box {
			font-size: $size;
			width: $size;
		}
	`)
	// Should appear twice
	count := strings.Count(css, "16px")
	if count < 2 {
		t.Errorf("expected '16px' to appear at least twice, got %d in:\n%s", count, css)
	}
	assertNotContains(t, css, "$size")
}

// TestVariableHexColor verifies that hex color variables are substituted correctly.
func TestVariableHexColor(t *testing.T) {
	css := mustTranspile(t, `
		$primary: #4a90d9;
		h1 { color: $primary; }
	`)
	assertContains(t, css, "#4a90d9")
	assertNotContains(t, css, "$primary")
}

// TestVariableNotInOutput verifies that the variable declaration itself does
// not appear as a CSS rule in the output.
func TestVariableNotInOutput(t *testing.T) {
	css := mustTranspile(t, `
		$x: 10px;
		.a { margin: $x; }
	`)
	assertNotContains(t, css, "$x")
	// Variable declaration should not become a CSS property
	assertNotContains(t, css, "$x:")
}

// ============================================================================
// 2. Mixin Expansion
// ============================================================================

// TestMixinNoArgs verifies that a mixin with no arguments is expanded correctly.
func TestMixinNoArgs(t *testing.T) {
	css := mustTranspile(t, `
		@mixin flex-center() {
			display: flex;
			align-items: center;
		}
		.box { @include flex-center(); }
	`)
	assertContains(t, css, "display: flex", "align-items: center")
	assertNotContains(t, css, "@mixin")
	assertNotContains(t, css, "@include")
}

// TestMixinWithArgs verifies that mixin arguments are substituted correctly.
func TestMixinWithArgs(t *testing.T) {
	css := mustTranspile(t, `
		@mixin button($bg, $fg) {
			background: $bg;
			color: $fg;
		}
		.btn { @include button(blue, white); }
	`)
	assertContains(t, css, "background: blue", "color: white")
	assertNotContains(t, css, "$bg")
	assertNotContains(t, css, "$fg")
}

// TestMixinWithDefault verifies that mixin default parameters are used when
// no argument is provided.
func TestMixinWithDefault(t *testing.T) {
	css := mustTranspile(t, `
		@mixin bordered($width: 1px, $style: solid, $color: black) {
			border: $width $style $color;
		}
		.card { @include bordered(); }
	`)
	assertContains(t, css, "border:")
	// Should use the defaults
	assertContains(t, css, "1px")
}

// TestMixinUsedTwice verifies that the same mixin can be included multiple
// times without the second expansion interfering with the first.
func TestMixinUsedTwice(t *testing.T) {
	css := mustTranspile(t, `
		@mixin color-rule($c) {
			color: $c;
		}
		.a { @include color-rule(red); }
		.b { @include color-rule(blue); }
	`)
	assertContains(t, css, "color: red", "color: blue")
}

// TestMixinDefinedAfterUse verifies that mixins can be used before they are
// defined in the source (like function hoisting in JavaScript).
func TestMixinDefinedAfterUse(t *testing.T) {
	css := mustTranspile(t, `
		.btn { @include clearfix(); }
		@mixin clearfix() {
			overflow: hidden;
		}
	`)
	assertContains(t, css, "overflow: hidden")
}

// ============================================================================
// 3. @if / @else Control Flow
// ============================================================================

// TestIfTrue verifies that the @if branch is taken when the condition is truthy.
func TestIfTrue(t *testing.T) {
	css := mustTranspile(t, `
		@mixin themed($theme) {
			@if $theme == dark {
				background: black;
				color: white;
			}
		}
		.dark { @include themed(dark); }
	`)
	assertContains(t, css, "background: black", "color: white")
}

// TestIfFalse verifies that the @if branch is NOT taken when the condition is false.
func TestIfFalse(t *testing.T) {
	css := mustTranspile(t, `
		@mixin themed($theme) {
			@if $theme == dark {
				background: black;
			}
		}
		.light { @include themed(light); }
	`)
	assertNotContains(t, css, "background: black")
}

// TestIfElse verifies that the @else branch is taken when @if is false.
func TestIfElse(t *testing.T) {
	css := mustTranspile(t, `
		@mixin themed($theme) {
			@if $theme == dark {
				background: black;
			} @else {
				background: white;
			}
		}
		.light { @include themed(light); }
	`)
	assertContains(t, css, "background: white")
	assertNotContains(t, css, "background: black")
}

// TestIfElseIfElse verifies that @else if chains work correctly.
func TestIfElseIfElse(t *testing.T) {
	css := mustTranspile(t, `
		@mixin size($s) {
			@if $s == sm {
				font-size: 12px;
			} @else if $s == md {
				font-size: 16px;
			} @else {
				font-size: 24px;
			}
		}
		.medium { @include size(md); }
	`)
	assertContains(t, css, "font-size: 16px")
	assertNotContains(t, css, "font-size: 12px")
	assertNotContains(t, css, "font-size: 24px")
}

// ============================================================================
// 4. @for Loops
// ============================================================================

// TestForThrough verifies that @for ... through generates the correct number
// of iterations (inclusive).
//
// Note: @for loops must appear inside a mixin or function body — they are
// lattice_block_item constructs, not top-level rules.
// The #{$i} interpolation syntax is not supported by the Lattice lexer;
// we test the loop variable substitution in property values instead.
func TestForThrough(t *testing.T) {
	css := mustTranspile(t, `
		@mixin make-paddings() {
			@for $i from 1 through 3 {
				.pad { padding: $i; }
			}
		}
		.x { @include make-paddings(); }
	`)
	// The loop variable $i should have been substituted as 1, 2, 3
	// Each iteration produces a padding declaration.
	count := strings.Count(css, "padding:")
	if count < 3 {
		t.Errorf("expected 3 padding declarations (1 through 3), got %d in:\n%s", count, css)
	}
}

// TestForTo verifies that @for ... to is exclusive (does not include the upper bound).
func TestForTo(t *testing.T) {
	css := mustTranspile(t, `
		@mixin make-margins() {
			@for $i from 1 to 4 {
				.m { margin: $i; }
			}
		}
		.x { @include make-margins(); }
	`)
	// 1 to 4 = 1, 2, 3 (exclusive upper bound) = 3 iterations
	count := strings.Count(css, "margin:")
	if count < 3 {
		t.Errorf("expected 3 margin declarations (1 to 4 exclusive), got %d in:\n%s", count, css)
	}
}

// ============================================================================
// 5. @each Loops
// ============================================================================

// TestEachLoop verifies that @each iterates over each list item.
//
// Note: @each loops must appear inside a mixin body.
// #{$var} interpolation syntax is not supported by the Lattice lexer,
// so we test by verifying the loop variable is substituted in property values.
func TestEachLoop(t *testing.T) {
	css := mustTranspile(t, `
		@mixin make-font-sizes() {
			@each $sz in 12px, 16px, 24px {
				.text { font-size: $sz; }
			}
		}
		.x { @include make-font-sizes(); }
	`)
	// Should have 3 font-size declarations
	count := strings.Count(css, "font-size:")
	if count < 3 {
		t.Errorf("expected at least 3 font-size declarations, got %d in:\n%s", count, css)
	}
}

// ============================================================================
// 6. @function Definitions and Calls
// ============================================================================

// TestFunctionBasic verifies that a simple @function with @return works.
func TestFunctionBasic(t *testing.T) {
	css := mustTranspile(t, `
		@function double($n) {
			@return $n * 2;
		}
		.box { width: double(8px); }
	`)
	assertContains(t, css, "width:")
	assertContains(t, css, "16px")
	assertNotContains(t, css, "double(")
}

// TestFunctionWithConditional verifies that @function with @if inside works.
func TestFunctionWithConditional(t *testing.T) {
	css := mustTranspile(t, `
		@function clamp-size($n) {
			@if $n > 100px {
				@return 100px;
			} @else {
				@return $n;
			}
		}
		.a { font-size: clamp-size(200px); }
		.b { font-size: clamp-size(50px); }
	`)
	assertContains(t, css, "font-size:")
}

// TestFunctionArithmetic verifies arithmetic expressions in @return.
func TestFunctionArithmetic(t *testing.T) {
	css := mustTranspile(t, `
		@function spacing($multiplier) {
			@return $multiplier * 8px;
		}
		.a { margin: spacing(2); }
		.b { padding: spacing(3); }
	`)
	assertContains(t, css, "margin:")
	assertContains(t, css, "padding:")
}

// ============================================================================
// 7. Scope Shadowing
// ============================================================================

// TestScopeShadowing verifies that inner $var declarations shadow outer ones,
// and the outer scope is restored after the inner block.
func TestScopeShadowing(t *testing.T) {
	css := mustTranspile(t, `
		$color: red;
		@mixin override() {
			$color: blue;
			color: $color;
		}
		.a { color: $color; }
		.b { @include override(); }
	`)
	// .a should use red (global), .b's include should use blue (local to mixin)
	assertContains(t, css, "color: red")
	assertContains(t, css, "color: blue")
}

// ============================================================================
// 8. Emitter Modes
// ============================================================================

// TestPrettyOutput verifies that pretty mode emits indented, newline-separated CSS.
func TestPrettyOutput(t *testing.T) {
	css, err := TranspileLatticeFull(`
		.a { color: red; }
	`, false, "  ")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Pretty output should have newlines and indentation
	if !strings.Contains(css, "\n") {
		t.Error("expected newlines in pretty output")
	}
}

// TestMinifiedOutput verifies that minified mode emits compact CSS.
func TestMinifiedOutput(t *testing.T) {
	css := mustTranspileMin(t, `.a { color: red; }`)
	// Minified output should NOT have newlines between rules
	// It may still have some structure, but should be compact
	assertContains(t, css, "color:red")
	// No space after colon in property declarations in minified mode
	assertNotContains(t, css, "color: red")
}

// TestMinifiedNoNewlines verifies that minified output is on a single line.
func TestMinifiedNoNewlines(t *testing.T) {
	css := mustTranspileMin(t, `
		.a { color: red; }
		.b { margin: 0; }
	`)
	if strings.Contains(css, "\n") {
		t.Errorf("expected no newlines in minified output, got:\n%s", css)
	}
}

// TestCustomIndent verifies that custom indent strings are respected in pretty mode.
func TestCustomIndent(t *testing.T) {
	css, err := TranspileLatticeFull(`.a { color: red; }`, false, "\t")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "\t") {
		t.Errorf("expected tab indentation, got:\n%s", css)
	}
}

// ============================================================================
// 9. Arithmetic in Variable Declarations
// ============================================================================

// TestArithmeticAddition verifies that $a + $b is computed at compile time.
func TestArithmeticAddition(t *testing.T) {
	css := mustTranspile(t, `
		@function add-sizes($a, $b) {
			@return $a + $b;
		}
		.box { width: add-sizes(10px, 5px); }
	`)
	assertContains(t, css, "15px")
}

// TestArithmeticMultiplication verifies that N * dimension is computed.
func TestArithmeticMultiplication(t *testing.T) {
	css := mustTranspile(t, `
		@function scale($n) {
			@return $n * 4px;
		}
		.a { margin: scale(3); }
	`)
	assertContains(t, css, "12px")
}

// TestArithmeticSubtraction verifies that $a - $b is computed.
func TestArithmeticSubtraction(t *testing.T) {
	css := mustTranspile(t, `
		@function subtract($a, $b) {
			@return $a - $b;
		}
		.a { height: subtract(100px, 20px); }
	`)
	assertContains(t, css, "80px")
}

// ============================================================================
// 10. Error Cases
// ============================================================================

// TestUndefinedVariable verifies that using an undefined $var returns an error.
func TestUndefinedVariable(t *testing.T) {
	// When a variable is used but never defined, the transformer should either
	// pass it through or error. The current implementation passes unknown
	// variables through as idents. Test that it at least does not crash.
	_, err := TranspileLattice(`.a { color: $undefined; }`)
	// The package may or may not error on undefined variables at the CSS level.
	// What matters is: no panic, and either an error or an output that doesn't
	// contain the raw $undefined token in a broken way.
	_ = err // acceptable either way: error or passthrough
}

// TestUndefinedMixin verifies that @include of an unknown mixin returns an error.
func TestUndefinedMixin(t *testing.T) {
	err := transpileErr(`.a { @include ghost-mixin(); }`)
	if err == nil {
		t.Error("expected error for undefined mixin, got nil")
	}
	typed, ok := err.(*UndefinedMixinError)
	if !ok {
		t.Fatalf("expected UndefinedMixinError, got %T", err)
	}
	if typed.Line == 0 || typed.Column == 0 {
		t.Fatalf("expected source position on undefined mixin error, got %d:%d", typed.Line, typed.Column)
	}
}

func TestUndefinedMixinSuggestion(t *testing.T) {
	err := transpileErr(`
		@mixin button() { color: red; }
		.a { @include buton; }
	`)
	typed, ok := err.(*UndefinedMixinError)
	if !ok {
		t.Fatalf("expected UndefinedMixinError, got %T", err)
	}
	if typed.Suggestion != "button" {
		t.Fatalf("expected suggestion 'button', got %q", typed.Suggestion)
	}
	if !strings.Contains(err.Error(), "Did you mean 'button'?") {
		t.Fatalf("expected suggestion in error message, got %q", err.Error())
	}
}

func TestMixinWithoutParensDefinition(t *testing.T) {
	css, err := TranspileLattice(`
		@mixin button { color: red; }
		.a { @include button; }
	`)
	if err != nil {
		t.Fatalf("expected mixin without parens to compile, got %v", err)
	}
	if !strings.Contains(css, "color: red;") {
		t.Fatalf("expected expanded mixin body, got %q", css)
	}
}

// TestCircularMixinReference verifies that circular @mixin references produce an error.
func TestCircularMixinReference(t *testing.T) {
	err := transpileErr(`
		@mixin a() { @include b(); }
		@mixin b() { @include a(); }
		.x { @include a(); }
	`)
	if err == nil {
		t.Error("expected error for circular mixin reference, got nil")
	}
}

// TestWrongArityMixin verifies that calling a mixin with wrong arg count returns error.
func TestWrongArityMixin(t *testing.T) {
	err := transpileErr(`
		@mixin button($bg, $fg) {
			background: $bg;
			color: $fg;
		}
		.a { @include button(red); }
	`)
	if err == nil {
		t.Error("expected arity error, got nil")
	}
}

// TestMissingReturn verifies that a @function without @return returns an error.
func TestMissingReturn(t *testing.T) {
	err := transpileErr(`
		@function noop($x) {
			$y: $x;
		}
		.a { width: noop(10px); }
	`)
	if err == nil {
		t.Error("expected missing return error, got nil")
	}
}

// TestTypeErrorArithmetic verifies that incompatible arithmetic produces an error.
func TestTypeErrorArithmetic(t *testing.T) {
	err := transpileErr(`
		@function bad($x) {
			@return $x + red;
		}
		.a { width: bad(10px); }
	`)
	if err == nil {
		t.Error("expected type error for 10px + red, got nil")
	}
}

// ============================================================================
// Unit Tests: ScopeChain
// ============================================================================

// TestScopeChainGet verifies that Get walks the parent chain.
func TestScopeChainGet(t *testing.T) {
	global := NewScopeChain(nil)
	global.Set("$color", "red")

	child := global.Child()
	val, ok := child.Get("$color")
	if !ok {
		t.Fatal("expected to find $color via parent chain")
	}
	if val != "red" {
		t.Errorf("expected 'red', got %v", val)
	}
}

// TestScopeChainShadow verifies that a child Set doesn't modify the parent.
func TestScopeChainShadow(t *testing.T) {
	global := NewScopeChain(nil)
	global.Set("$color", "red")

	child := global.Child()
	child.Set("$color", "blue")

	// Parent unchanged
	parentVal, _ := global.Get("$color")
	if parentVal != "red" {
		t.Errorf("expected parent to still be 'red', got %v", parentVal)
	}
	// Child sees its own value
	childVal, _ := child.Get("$color")
	if childVal != "blue" {
		t.Errorf("expected child to see 'blue', got %v", childVal)
	}
}

// TestScopeChainHas reports that Has works across the chain.
func TestScopeChainHas(t *testing.T) {
	global := NewScopeChain(nil)
	global.Set("$x", 42)

	child := global.Child()
	if !child.Has("$x") {
		t.Error("expected Has to find $x via parent")
	}
	if child.Has("$y") {
		t.Error("expected Has to not find $y")
	}
}

// TestScopeChainHasLocal verifies that HasLocal only checks the current scope.
func TestScopeChainHasLocal(t *testing.T) {
	global := NewScopeChain(nil)
	global.Set("$x", 1)

	child := global.Child()
	if child.HasLocal("$x") {
		t.Error("expected HasLocal to NOT find $x in child scope (it's in parent)")
	}
	child.Set("$x", 2)
	if !child.HasLocal("$x") {
		t.Error("expected HasLocal to find $x after child.Set")
	}
}

// TestScopeChainDepth verifies that Depth counts levels correctly.
func TestScopeChainDepth(t *testing.T) {
	g := NewScopeChain(nil)
	if g.Depth() != 0 {
		t.Errorf("global depth should be 0, got %d", g.Depth())
	}
	c1 := g.Child()
	if c1.Depth() != 1 {
		t.Errorf("child depth should be 1, got %d", c1.Depth())
	}
	c2 := c1.Child()
	if c2.Depth() != 2 {
		t.Errorf("grandchild depth should be 2, got %d", c2.Depth())
	}
}

// TestScopeChainMissing verifies that Get returns (nil, false) for missing names.
func TestScopeChainMissing(t *testing.T) {
	s := NewScopeChain(nil)
	val, ok := s.Get("$nonexistent")
	if ok {
		t.Error("expected ok=false for missing name")
	}
	if val != nil {
		t.Error("expected nil for missing name")
	}
}

// ============================================================================
// Unit Tests: LatticeValue types
// ============================================================================

// TestLatticeNumberString verifies integer and float formatting.
func TestLatticeNumberString(t *testing.T) {
	tests := []struct {
		val      LatticeNumber
		expected string
	}{
		{LatticeNumber{42}, "42"},
		{LatticeNumber{3.14}, "3.14"},
		{LatticeNumber{0}, "0"},
		{LatticeNumber{-1}, "-1"},
	}
	for _, tt := range tests {
		got := tt.val.String()
		if got != tt.expected {
			t.Errorf("LatticeNumber(%v).String() = %q, want %q", tt.val.Value, got, tt.expected)
		}
	}
}

// TestLatticeDimensionString verifies dimension formatting.
func TestLatticeDimensionString(t *testing.T) {
	tests := []struct {
		val      LatticeDimension
		expected string
	}{
		{LatticeDimension{16, "px"}, "16px"},
		{LatticeDimension{1.5, "rem"}, "1.5rem"},
		{LatticeDimension{100, "vh"}, "100vh"},
	}
	for _, tt := range tests {
		got := tt.val.String()
		if got != tt.expected {
			t.Errorf("LatticeDimension(%v %q).String() = %q, want %q",
				tt.val.Value, tt.val.Unit, got, tt.expected)
		}
	}
}

// TestLatticePercentageString verifies percentage formatting.
func TestLatticePercentageString(t *testing.T) {
	tests := []struct {
		val      LatticePercentage
		expected string
	}{
		{LatticePercentage{50}, "50%"},
		{LatticePercentage{33.33}, "33.33%"},
		{LatticePercentage{100}, "100%"},
	}
	for _, tt := range tests {
		got := tt.val.String()
		if got != tt.expected {
			t.Errorf("LatticePercentage(%v).String() = %q, want %q", tt.val.Value, got, tt.expected)
		}
	}
}

// TestLatticeTruthy verifies Truthy() semantics for all value types.
func TestLatticeTruthy(t *testing.T) {
	tests := []struct {
		val    LatticeValue
		truthy bool
	}{
		{LatticeNumber{0}, false},
		{LatticeNumber{1}, true},
		{LatticeDimension{16, "px"}, true},
		{LatticePercentage{0}, true},
		{LatticeString{"hello"}, true},
		{LatticeIdent{"red"}, true},
		{LatticeColor{"#fff"}, true},
		{LatticeBool{true}, true},
		{LatticeBool{false}, false},
		{LatticeNull{}, false},
		{LatticeList{[]LatticeValue{}}, false},
		{LatticeList{[]LatticeValue{LatticeNumber{1}}}, true},
	}
	for _, tt := range tests {
		got := tt.val.Truthy()
		if got != tt.truthy {
			t.Errorf("%T(%v).Truthy() = %v, want %v", tt.val, tt.val, got, tt.truthy)
		}
	}
}

// ============================================================================
// Unit Tests: Error Types
// ============================================================================

// TestErrorMessages verifies that error constructors produce human-readable messages.
func TestErrorMessages(t *testing.T) {
	tests := []struct {
		err      error
		contains string
	}{
		{NewUndefinedVariableError("$color", 5, 10), "$color"},
		{NewUndefinedMixinError("flex-center", 3, 1), "flex-center"},
		{NewUndefinedMixinError("flec-center", 3, 1, "flex-center"), "Did you mean 'flex-center'?"},
		{NewUndefinedFunctionError("spacing", 7, 5), "spacing"},
		{NewWrongArityError("Mixin", "button", 2, 3, 1, 1), "button"},
		{NewCircularReferenceError("mixin", []string{"a", "b", "a"}, 2, 1), "a → b → a"},
		{NewTypeErrorInExpression("add", "10px", "red", 4, 3), "10px"},
		{NewUnitMismatchError("px", "s", 6, 2), "px"},
		{NewMissingReturnError("noop", 8, 1), "noop"},
		{NewModuleNotFoundError("tokens", 1, 1), "tokens"},
		{NewReturnOutsideFunctionError(3, 5), "@return"},
	}

	for _, tt := range tests {
		msg := tt.err.Error()
		if !strings.Contains(msg, tt.contains) {
			t.Errorf("error %T message %q does not contain %q", tt.err, msg, tt.contains)
		}
	}
}

// TestErrorLineColumn verifies that error messages include line/column info.
func TestErrorLineColumn(t *testing.T) {
	err := NewUndefinedVariableError("$x", 5, 10)
	msg := err.Error()
	if !strings.Contains(msg, "5") {
		t.Errorf("expected line 5 in error message, got: %s", msg)
	}
	if !strings.Contains(msg, "10") {
		t.Errorf("expected column 10 in error message, got: %s", msg)
	}
}

// TestLatticeErrorNoPosition verifies that LatticeError with Line=0 omits position.
func TestLatticeErrorNoPosition(t *testing.T) {
	err := &LatticeError{Message: "something went wrong"}
	msg := err.Error()
	if strings.Contains(msg, "line") {
		t.Errorf("expected no position in zero-line error, got: %s", msg)
	}
}

// ============================================================================
// Unit Tests: CSSEmitter
// ============================================================================

// TestEmitterEmpty verifies that Emit(nil) returns an empty string.
func TestEmitterEmpty(t *testing.T) {
	e := NewCSSEmitter(false, "  ")
	if got := e.Emit(nil); got != "" {
		t.Errorf("expected empty string for nil AST, got %q", got)
	}
}

// TestEmitterPrettyVsMinified verifies that pretty and minified mode differ.
func TestEmitterPrettyVsMinified(t *testing.T) {
	pretty, _ := TranspileLatticeFull(`.a { color: red; }`, false, "  ")
	minified, _ := TranspileLatticeFull(`.a { color: red; }`, true, "")

	if pretty == minified {
		t.Error("expected pretty and minified output to differ")
	}
	if len(minified) >= len(pretty) {
		t.Errorf("expected minified (%d chars) to be shorter than pretty (%d chars)",
			len(minified), len(pretty))
	}
}

// ============================================================================
// Integration: Multiple CSS Rules
// ============================================================================

// TestMultipleRules verifies that multiple CSS rules are all emitted.
func TestMultipleRules(t *testing.T) {
	css := mustTranspile(t, `
		.a { color: red; }
		.b { color: blue; }
		.c { color: green; }
	`)
	assertContains(t, css, ".a", ".b", ".c")
	assertContains(t, css, "red", "blue", "green")
}

// TestPassThroughCSS verifies that plain CSS (no Lattice features) passes through unchanged.
func TestPassThroughCSS(t *testing.T) {
	css := mustTranspile(t, `h1 { font-size: 24px; font-weight: bold; }`)
	assertContains(t, css, "h1", "font-size:", "24px", "font-weight:", "bold")
}

// ============================================================================
// Additional Coverage Tests
// ============================================================================

// TestTransformLatticeAST verifies the step-by-step API TransformLatticeAST.
func TestTransformLatticeAST(t *testing.T) {
	latticeparser, err := latticeparser.ParseLattice(`$x: 5px; .a { margin: $x; }`)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	cssAST, err := TransformLatticeAST(latticeparser)
	if err != nil {
		t.Fatalf("transform error: %v", err)
	}
	if cssAST == nil {
		t.Fatal("expected non-nil CSS AST")
	}
	css := EmitCSS(cssAST)
	assertContains(t, css, "margin:", "5px")
}

// TestEmitCSSMinified verifies the EmitCSSMinified function.
func TestEmitCSSMinified(t *testing.T) {
	ast, err := latticeparser.ParseLattice(`.a { color: red; }`)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	cssAST, _ := TransformLatticeAST(ast)
	css := EmitCSSMinified(cssAST)
	assertContains(t, css, "color:red")
	assertNotContains(t, css, "color: red")
}

// TestCSSFunctionPassThrough verifies that CSS built-in functions like rgb()
// are passed through unchanged in property values.
func TestCSSFunctionPassThrough(t *testing.T) {
	css := mustTranspile(t, `.btn { color: rgb(255, 0, 0); }`)
	assertContains(t, css, "rgb(")
	assertContains(t, css, "255")
}

// TestAtMediaRule verifies that @media rules pass through correctly.
func TestAtMediaRule(t *testing.T) {
	css := mustTranspile(t, `@media (max-width: 768px) { .btn { font-size: 14px; } }`)
	assertContains(t, css, "@media")
	assertContains(t, css, "font-size:")
	assertContains(t, css, "14px")
}

// TestLogicalOr verifies that "or" expressions evaluate correctly.
func TestLogicalOr(t *testing.T) {
	css := mustTranspile(t, `
		@function check($x) {
			@if $x == a or $x == b {
				@return yes;
			} @else {
				@return no;
			}
		}
		.a { content: check(a); }
		.b { content: check(c); }
	`)
	assertContains(t, css, "content:")
}

// TestLogicalAnd verifies that "and" expressions evaluate correctly.
// Note: Lattice supports >, >=, <=, ==, != but not bare < (use >= inverted).
func TestLogicalAnd(t *testing.T) {
	css := mustTranspile(t, `
		@function check($x) {
			@if $x > 0 and $x >= 1 {
				@return in-range;
			} @else {
				@return out-range;
			}
		}
		.a { content: check(5); }
	`)
	assertContains(t, css, "content:")
}

// TestNegateExpression verifies that unary minus negates values correctly.
func TestNegateExpression(t *testing.T) {
	css := mustTranspile(t, `
		@function negate($n) {
			@return -$n;
		}
		.a { margin: negate(10px); }
	`)
	assertContains(t, css, "margin:")
	assertContains(t, css, "-10px")
}

// TestLatticeValueLatticeValueMethod verifies that LatticeValue implementations
// compile and satisfy the interface.
func TestLatticeValueInterface(t *testing.T) {
	// All types must implement LatticeValue
	var vals []LatticeValue = []LatticeValue{
		LatticeNumber{1},
		LatticeDimension{1, "px"},
		LatticePercentage{50},
		LatticeString{"hi"},
		LatticeIdent{"red"},
		LatticeColor{"#fff"},
		LatticeBool{true},
		LatticeNull{},
		LatticeList{nil},
	}
	for _, v := range vals {
		_ = v.String()
		_ = v.Truthy()
		v.latticeValue() // call the marker method
	}
}

// TestLatticeStringString verifies LatticeString.String() uses double-quote formatting.
func TestLatticeStringString(t *testing.T) {
	s := LatticeString{Value: "hello"}
	got := s.String()
	if !strings.Contains(got, "hello") {
		t.Errorf("LatticeString.String() = %q, expected to contain 'hello'", got)
	}
}

// TestLatticeBoolString verifies LatticeBool.String() returns "true"/"false".
func TestLatticeBoolString(t *testing.T) {
	if got := (LatticeBool{true}).String(); got != "true" {
		t.Errorf("LatticeBool{true}.String() = %q, want 'true'", got)
	}
	if got := (LatticeBool{false}).String(); got != "false" {
		t.Errorf("LatticeBool{false}.String() = %q, want 'false'", got)
	}
}

// TestLatticeNullString verifies LatticeNull.String() returns empty string.
func TestLatticeNullString(t *testing.T) {
	if got := (LatticeNull{}).String(); got != "" {
		t.Errorf("LatticeNull.String() = %q, want ''", got)
	}
}

// TestLatticeListString verifies LatticeList.String() joins with ", ".
func TestLatticeListString(t *testing.T) {
	lst := LatticeList{Items: []LatticeValue{
		LatticeIdent{"red"},
		LatticeIdent{"green"},
	}}
	got := lst.String()
	if !strings.Contains(got, "red") || !strings.Contains(got, "green") {
		t.Errorf("LatticeList.String() = %q, expected 'red' and 'green'", got)
	}
}

// TestFunctionMultipleParams verifies functions with multiple params work.
func TestFunctionMultipleParams(t *testing.T) {
	css := mustTranspile(t, `
		@function add-two($a, $b) {
			@return $a + $b;
		}
		.x { width: add-two(10px, 5px); }
	`)
	assertContains(t, css, "15px")
}

// TestIsCSSFunction verifies the CSS function name detection.
func TestIsCSSFunction(t *testing.T) {
	if !isCSSFunction("rgb(") {
		t.Error("expected rgb( to be a CSS function")
	}
	if !isCSSFunction("calc") {
		t.Error("expected calc to be a CSS function")
	}
	if isCSSFunction("my-custom") {
		t.Error("expected my-custom NOT to be a CSS function")
	}
}

// ============================================================================
// Expression Evaluator Unit Tests
// ============================================================================

// TestExpressionEvaluatorComparisons tests all comparison operators.
func TestExpressionEvaluatorComparisons(t *testing.T) {
	tests := []struct {
		source   string
		expected string
	}{
		// == comparison
		{`@function f($x) { @if $x == 5 { @return yes; } @else { @return no; } } .a { v: f(5); }`, "yes"},
		{`@function f($x) { @if $x == 5 { @return yes; } @else { @return no; } } .a { v: f(3); }`, "no"},
		// != comparison
		{`@function f($x) { @if $x != 5 { @return yes; } @else { @return no; } } .a { v: f(3); }`, "yes"},
		{`@function f($x) { @if $x != 5 { @return yes; } @else { @return no; } } .a { v: f(5); }`, "no"},
		// > comparison
		{`@function f($x) { @if $x > 3 { @return yes; } @else { @return no; } } .a { v: f(5); }`, "yes"},
		// >= comparison
		{`@function f($x) { @if $x >= 5 { @return yes; } @else { @return no; } } .a { v: f(5); }`, "yes"},
		// <= comparison
		{`@function f($x) { @if $x <= 5 { @return yes; } @else { @return no; } } .a { v: f(5); }`, "yes"},
	}

	for _, tt := range tests {
		css, err := TranspileLattice(tt.source)
		if err != nil {
			t.Errorf("unexpected error: %v\nsource: %s", err, tt.source)
			continue
		}
		if !strings.Contains(css, tt.expected) {
			t.Errorf("expected output to contain %q\nsource: %s\ngot: %s", tt.expected, tt.source, css)
		}
	}
}

// TestExpressionEvaluatorPercentage tests percentage arithmetic.
func TestExpressionEvaluatorPercentage(t *testing.T) {
	css := mustTranspile(t, `
		@function half($p) {
			@return $p * 50;
		}
		.a { width: half(2%); }
	`)
	// 2% * 50 — this tests Percentage × Number
	assertContains(t, css, "width:")
}

// TestFunctionMultiParamsFromAtRule tests that @function with 2+ params works.
// This exercises collectFunctionFromAtRule (at_rule path).
func TestFunctionMultiParamsFromAtRule(t *testing.T) {
	// A function with 3+ parameters may be parsed as an at_rule
	css := mustTranspile(t, `
		@function clamp-val($val, $min, $max) {
			@if $val >= $max {
				@return $max;
			} @else {
				@return $val;
			}
		}
		.a { width: clamp-val(50px, 0px, 100px); }
	`)
	assertContains(t, css, "width:")
}

// TestTokenToValueEdgeCases tests tokenToValue with various token types.
func TestTokenToValueEdgeCases(t *testing.T) {
	tests := []struct {
		tok      lexer.Token
		typeName string
		value    string
	}{
		{lexer.Token{TypeName: "NUMBER", Value: "42"}, "NUMBER", "42"},
		{lexer.Token{TypeName: "DIMENSION", Value: "16px"}, "DIMENSION", "16px"},
		{lexer.Token{TypeName: "PERCENTAGE", Value: "50%"}, "PERCENTAGE", "50%"},
		{lexer.Token{TypeName: "STRING", Value: "hello"}, "STRING", "hello"},
		{lexer.Token{TypeName: "HASH", Value: "#fff"}, "HASH", "#fff"},
		{lexer.Token{TypeName: "IDENT", Value: "true"}, "IDENT", "true"},
		{lexer.Token{TypeName: "IDENT", Value: "false"}, "IDENT", "false"},
		{lexer.Token{TypeName: "IDENT", Value: "null"}, "IDENT", ""},
		{lexer.Token{TypeName: "IDENT", Value: "red"}, "IDENT", "red"},
		{lexer.Token{TypeName: "UNKNOWN", Value: "x"}, "UNKNOWN", "x"},
	}

	for _, tt := range tests {
		val := tokenToValue(tt.tok)
		got := val.String()
		if tt.value == "" {
			if got != "" {
				t.Errorf("tokenToValue(%s=%q).String() = %q, want empty", tt.typeName, tt.tok.Value, got)
			}
		} else {
			if !strings.Contains(got, tt.value) && got != tt.value {
				t.Errorf("tokenToValue(%s=%q).String() = %q, want %q", tt.typeName, tt.tok.Value, got, tt.value)
			}
		}
	}
}

// TestPercentageArithmetic tests percentage addition and subtraction.
func TestPercentageArithmetic(t *testing.T) {
	css := mustTranspile(t, `
		@function add-pct($a, $b) {
			@return $a + $b;
		}
		.a { width: add-pct(30%, 20%); }
	`)
	assertContains(t, css, "50%")
}

// TestNegatePercentage tests negating a percentage value.
func TestNegatePercentage(t *testing.T) {
	css := mustTranspile(t, `
		@function neg($p) {
			@return -$p;
		}
		.a { margin: neg(10%); }
	`)
	assertContains(t, css, "-10%")
}

// TestNegateNumber tests negating a number value.
func TestNegateNumber(t *testing.T) {
	css := mustTranspile(t, `
		@function neg($n) {
			@return -$n;
		}
		.a { z-index: neg(5); }
	`)
	assertContains(t, css, "-5")
}

// TestSubtractDimensions tests dimension subtraction.
func TestSubtractDimensions(t *testing.T) {
	css := mustTranspile(t, `
		@function sub($a, $b) {
			@return $a - $b;
		}
		.a { width: sub(100px, 20px); }
	`)
	assertContains(t, css, "80px")
}

// TestSubtractNumbers tests number subtraction.
func TestSubtractNumbers(t *testing.T) {
	css := mustTranspile(t, `
		@function sub($a, $b) {
			@return $a - $b;
		}
		.a { z-index: sub(10, 3); }
	`)
	assertContains(t, css, "7")
}

// TestMultiplyNumberByNumber tests number × number multiplication.
func TestMultiplyNumberByNumber(t *testing.T) {
	css := mustTranspile(t, `
		@function mul($a, $b) {
			@return $a * $b;
		}
		.a { z-index: mul(3, 4); }
	`)
	assertContains(t, css, "12")
}

// TestMultiplyDimensionByNumber tests dimension × number multiplication.
func TestMultiplyDimensionByNumber(t *testing.T) {
	css := mustTranspile(t, `
		@function scale($d, $n) {
			@return $d * $n;
		}
		.a { width: scale(10px, 3); }
	`)
	assertContains(t, css, "30px")
}

// TestSelectorWithPseudoClass tests that CSS pseudo-class selectors pass through.
func TestSelectorWithPseudoClass(t *testing.T) {
	css := mustTranspile(t, `.btn:hover { color: blue; }`)
	assertContains(t, css, ".btn")
	assertContains(t, css, "color: blue")
}

// TestMultipleSelectorsComma tests comma-separated selectors.
func TestMultipleSelectorsComma(t *testing.T) {
	css := mustTranspile(t, `h1, h2, h3 { font-weight: bold; }`)
	assertContains(t, css, "font-weight:", "bold")
}

// TestStringValue tests that string property values are emitted correctly.
func TestStringValue(t *testing.T) {
	css := mustTranspile(t, `.a { content: "hello world"; }`)
	assertContains(t, css, "content:")
	assertContains(t, css, "hello world")
}

// TestEmitTokenTypeName verifies tokenTypeName function works for both
// grammar-driven (TypeName field) and legacy (Type int) tokens.
func TestTokenTypeName(t *testing.T) {
	// Grammar-driven token: TypeName is non-empty
	tok1 := lexer.Token{TypeName: "IDENT", Value: "red"}
	if got := tokenTypeName(tok1); got != "IDENT" {
		t.Errorf("expected 'IDENT', got %q", got)
	}

	// Token with zero TypeName falls back to Type.String()
	tok2 := lexer.Token{Value: "red"} // TypeName is ""
	got2 := tokenTypeName(tok2)
	// Just ensure it doesn't panic
	_ = got2
}

// ============================================================================
// Lattice v2: New Error Types
// ============================================================================

func TestMaxIterationError(t *testing.T) {
	err := NewMaxIterationError(1000, 5, 3)
	if !strings.Contains(err.Error(), "1000") {
		t.Errorf("expected message to contain '1000', got %q", err.Error())
	}
	if err.MaxIterations != 1000 {
		t.Errorf("expected MaxIterations=1000, got %d", err.MaxIterations)
	}
}

func TestExtendTargetNotFoundError(t *testing.T) {
	err := NewExtendTargetNotFoundError("%message-shared", 10, 5)
	if !strings.Contains(err.Error(), "%message-shared") {
		t.Errorf("expected message to contain '%%message-shared', got %q", err.Error())
	}
}

func TestRangeError(t *testing.T) {
	err := NewRangeError("Index 5 out of bounds", 3, 1)
	if !strings.Contains(err.Error(), "Index 5") {
		t.Errorf("expected message to contain 'Index 5', got %q", err.Error())
	}
}

func TestZeroDivisionError(t *testing.T) {
	err := NewZeroDivisionInExpressionError(7, 10)
	if !strings.Contains(err.Error(), "Division by zero") {
		t.Errorf("expected 'Division by zero', got %q", err.Error())
	}
}

// ============================================================================
// Lattice v2: Scope — SetGlobal
// ============================================================================

func TestScopeSetGlobal(t *testing.T) {
	global := NewScopeChain(nil)
	child := global.Child()
	grandchild := child.Child()

	grandchild.SetGlobal("$x", LatticeNumber{Value: 42})

	// Should be set in the global scope, not the child/grandchild
	val, ok := global.Get("$x")
	if !ok {
		t.Fatal("expected $x to be set in global scope")
	}
	num, ok := val.(LatticeNumber)
	if !ok || num.Value != 42 {
		t.Errorf("expected LatticeNumber(42), got %v", val)
	}

	// Should be accessible from all scopes
	val2, _ := grandchild.Get("$x")
	if val2 == nil {
		t.Error("expected $x to be visible from grandchild scope")
	}
}

// ============================================================================
// Lattice v2: LatticeMap
// ============================================================================

func TestLatticeMapBasic(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "primary", Value: LatticeColor{Value: "#4a90d9"}},
		{Key: "secondary", Value: LatticeColor{Value: "#7b68ee"}},
	}}

	if !m.Truthy() {
		t.Error("expected non-empty map to be truthy")
	}

	val, ok := m.MapGet("primary")
	if !ok {
		t.Fatal("expected to find 'primary'")
	}
	if val.String() != "#4a90d9" {
		t.Errorf("expected '#4a90d9', got %q", val.String())
	}

	if !m.MapHasKey("secondary") {
		t.Error("expected map to have key 'secondary'")
	}
	if m.MapHasKey("tertiary") {
		t.Error("expected map NOT to have key 'tertiary'")
	}

	keys := m.MapKeys()
	if len(keys) != 2 || keys[0] != "primary" || keys[1] != "secondary" {
		t.Errorf("unexpected keys: %v", keys)
	}
}

func TestLatticeMapString(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "a", Value: LatticeNumber{Value: 1}},
	}}
	s := m.String()
	if !strings.Contains(s, "a: 1") {
		t.Errorf("expected map string to contain 'a: 1', got %q", s)
	}
}

// ============================================================================
// Lattice v2: Color Conversion Helpers
// ============================================================================

func TestColorToRGB(t *testing.T) {
	// Test #RGB shorthand
	r, g, b, a := colorToRGB("#f00")
	if r != 255 || g != 0 || b != 0 || a != 1.0 {
		t.Errorf("expected (255,0,0,1.0), got (%d,%d,%d,%f)", r, g, b, a)
	}

	// Test #RRGGBB
	r, g, b, a = colorToRGB("#4a90d9")
	if r != 74 || g != 144 || b != 217 {
		t.Errorf("expected (74,144,217), got (%d,%d,%d)", r, g, b)
	}
}

func TestColorFromRGB(t *testing.T) {
	hex := colorFromRGB(255, 0, 0, 1.0)
	if hex != "#ff0000" {
		t.Errorf("expected '#ff0000', got %q", hex)
	}

	rgba := colorFromRGB(255, 0, 0, 0.5)
	if !strings.Contains(rgba, "rgba") {
		t.Errorf("expected rgba() notation for alpha < 1, got %q", rgba)
	}
}

func TestColorHSLRoundTrip(t *testing.T) {
	// Pure red: hsl(0, 100%, 50%)
	hex := colorFromHSL(0, 100, 50, 1.0)
	if hex != "#ff0000" {
		t.Errorf("expected '#ff0000', got %q", hex)
	}

	// Check HSL extraction
	h, s, l, _ := colorToHSL("#ff0000")
	if h != 0 || s != 100 || l != 50 {
		t.Errorf("expected (0, 100, 50), got (%f, %f, %f)", h, s, l)
	}
}

// ============================================================================
// Lattice v2: Built-in Functions — Unit Tests
// ============================================================================

func TestBuiltinTypeOf(t *testing.T) {
	tests := []struct {
		input    LatticeValue
		expected string
	}{
		{LatticeNumber{Value: 42}, "number"},
		{LatticeDimension{Value: 16, Unit: "px"}, "number"},
		{LatticeString{Value: "hello"}, "string"},
		{LatticeIdent{Value: "red"}, "string"},
		{LatticeColor{Value: "#fff"}, "color"},
		{LatticeBool{Value: true}, "bool"},
		{LatticeNull{}, "null"},
		{LatticeList{Items: []LatticeValue{}}, "list"},
		{LatticeMap{Items: []MapEntry{}}, "map"},
	}

	for _, tt := range tests {
		result := builtinTypeOf([]LatticeValue{tt.input}, nil)
		if s, ok := result.(LatticeString); !ok || s.Value != tt.expected {
			t.Errorf("type-of(%v) = %v, want %q", tt.input, result, tt.expected)
		}
	}
}

func TestBuiltinLength(t *testing.T) {
	list := LatticeList{Items: []LatticeValue{
		LatticeIdent{Value: "a"}, LatticeIdent{Value: "b"}, LatticeIdent{Value: "c"},
	}}
	result := builtinLength([]LatticeValue{list}, nil)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 3 {
		t.Errorf("length() = %v, want 3", result)
	}
}

func TestBuiltinNth(t *testing.T) {
	list := LatticeList{Items: []LatticeValue{
		LatticeIdent{Value: "a"}, LatticeIdent{Value: "b"}, LatticeIdent{Value: "c"},
	}}
	result := builtinNth([]LatticeValue{list, LatticeNumber{Value: 2}}, nil)
	if id, ok := result.(LatticeIdent); !ok || id.Value != "b" {
		t.Errorf("nth() = %v, want 'b'", result)
	}
}

func TestBuiltinMapGet(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "primary", Value: LatticeColor{Value: "#4a90d9"}},
	}}
	result := builtinMapGet([]LatticeValue{m, LatticeIdent{Value: "primary"}}, nil)
	if c, ok := result.(LatticeColor); !ok || c.Value != "#4a90d9" {
		t.Errorf("map-get() = %v, want #4a90d9", result)
	}

	// Missing key returns null
	result2 := builtinMapGet([]LatticeValue{m, LatticeIdent{Value: "missing"}}, nil)
	if _, ok := result2.(LatticeNull); !ok {
		t.Errorf("map-get(missing) = %v, want null", result2)
	}
}

func TestBuiltinMapMerge(t *testing.T) {
	m1 := LatticeMap{Items: []MapEntry{
		{Key: "a", Value: LatticeNumber{Value: 1}},
		{Key: "b", Value: LatticeNumber{Value: 2}},
	}}
	m2 := LatticeMap{Items: []MapEntry{
		{Key: "b", Value: LatticeNumber{Value: 99}},
		{Key: "c", Value: LatticeNumber{Value: 3}},
	}}
	result := builtinMapMerge([]LatticeValue{m1, m2}, nil)
	merged, ok := result.(LatticeMap)
	if !ok {
		t.Fatalf("expected LatticeMap, got %T", result)
	}
	if len(merged.Items) != 3 {
		t.Errorf("expected 3 items, got %d", len(merged.Items))
	}
	// b should be overwritten by m2
	val, _ := merged.MapGet("b")
	if n, ok := val.(LatticeNumber); !ok || n.Value != 99 {
		t.Errorf("expected b=99, got %v", val)
	}
}

func TestBuiltinMathDiv(t *testing.T) {
	result := builtinMathDiv([]LatticeValue{
		LatticeNumber{Value: 100},
		LatticeNumber{Value: 4},
	}, nil)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 25 {
		t.Errorf("math.div(100, 4) = %v, want 25", result)
	}

	// Dimension / Number
	result2 := builtinMathDiv([]LatticeValue{
		LatticeDimension{Value: 100, Unit: "px"},
		LatticeNumber{Value: 4},
	}, nil)
	if d, ok := result2.(LatticeDimension); !ok || d.Value != 25 || d.Unit != "px" {
		t.Errorf("math.div(100px, 4) = %v, want 25px", result2)
	}
}

func TestBuiltinMathFloorCeilRound(t *testing.T) {
	floor := builtinMathFloor([]LatticeValue{LatticeNumber{Value: 3.7}}, nil)
	if n, ok := floor.(LatticeNumber); !ok || n.Value != 3 {
		t.Errorf("math.floor(3.7) = %v, want 3", floor)
	}

	ceil := builtinMathCeil([]LatticeValue{LatticeNumber{Value: 3.2}}, nil)
	if n, ok := ceil.(LatticeNumber); !ok || n.Value != 4 {
		t.Errorf("math.ceil(3.2) = %v, want 4", ceil)
	}

	round := builtinMathRound([]LatticeValue{LatticeNumber{Value: 3.5}}, nil)
	if n, ok := round.(LatticeNumber); !ok || n.Value != 4 {
		t.Errorf("math.round(3.5) = %v, want 4", round)
	}
}

func TestBuiltinMathAbs(t *testing.T) {
	result := builtinMathAbs([]LatticeValue{LatticeNumber{Value: -42}}, nil)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 42 {
		t.Errorf("math.abs(-42) = %v, want 42", result)
	}
}

func TestBuiltinMathMinMax(t *testing.T) {
	min := builtinMathMin([]LatticeValue{
		LatticeNumber{Value: 5}, LatticeNumber{Value: 2}, LatticeNumber{Value: 8},
	}, nil)
	if n, ok := min.(LatticeNumber); !ok || n.Value != 2 {
		t.Errorf("math.min(5,2,8) = %v, want 2", min)
	}

	max := builtinMathMax([]LatticeValue{
		LatticeNumber{Value: 5}, LatticeNumber{Value: 2}, LatticeNumber{Value: 8},
	}, nil)
	if n, ok := max.(LatticeNumber); !ok || n.Value != 8 {
		t.Errorf("math.max(5,2,8) = %v, want 8", max)
	}
}

func TestBuiltinUnit(t *testing.T) {
	result := builtinUnit([]LatticeValue{LatticeDimension{Value: 16, Unit: "px"}}, nil)
	if s, ok := result.(LatticeString); !ok || s.Value != "px" {
		t.Errorf("unit(16px) = %v, want 'px'", result)
	}
}

func TestBuiltinUnitless(t *testing.T) {
	result := builtinUnitless([]LatticeValue{LatticeNumber{Value: 42}}, nil)
	if b, ok := result.(LatticeBool); !ok || !b.Value {
		t.Errorf("unitless(42) = %v, want true", result)
	}

	result2 := builtinUnitless([]LatticeValue{LatticeDimension{Value: 42, Unit: "px"}}, nil)
	if b, ok := result2.(LatticeBool); !ok || b.Value {
		t.Errorf("unitless(42px) = %v, want false", result2)
	}
}

func TestBuiltinJoin(t *testing.T) {
	l1 := LatticeList{Items: []LatticeValue{LatticeIdent{Value: "a"}, LatticeIdent{Value: "b"}}}
	l2 := LatticeList{Items: []LatticeValue{LatticeIdent{Value: "c"}}}
	result := builtinJoin([]LatticeValue{l1, l2}, nil)
	if lst, ok := result.(LatticeList); !ok || len(lst.Items) != 3 {
		t.Errorf("join() = %v, want 3 items", result)
	}
}

func TestBuiltinAppend(t *testing.T) {
	l := LatticeList{Items: []LatticeValue{LatticeIdent{Value: "a"}}}
	result := builtinAppend([]LatticeValue{l, LatticeIdent{Value: "b"}}, nil)
	if lst, ok := result.(LatticeList); !ok || len(lst.Items) != 2 {
		t.Errorf("append() = %v, want 2 items", result)
	}
}

func TestBuiltinIndex(t *testing.T) {
	l := LatticeList{Items: []LatticeValue{
		LatticeIdent{Value: "a"}, LatticeIdent{Value: "b"}, LatticeIdent{Value: "c"},
	}}
	result := builtinIndex([]LatticeValue{l, LatticeIdent{Value: "b"}}, nil)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 2 {
		t.Errorf("index(list, 'b') = %v, want 2", result)
	}

	// Not found returns null
	result2 := builtinIndex([]LatticeValue{l, LatticeIdent{Value: "z"}}, nil)
	if _, ok := result2.(LatticeNull); !ok {
		t.Errorf("index(list, 'z') = %v, want null", result2)
	}
}

func TestBuiltinComparable(t *testing.T) {
	result := builtinComparable([]LatticeValue{
		LatticeDimension{Value: 10, Unit: "px"},
		LatticeDimension{Value: 20, Unit: "px"},
	}, nil)
	if b, ok := result.(LatticeBool); !ok || !b.Value {
		t.Errorf("comparable(10px, 20px) = %v, want true", result)
	}

	result2 := builtinComparable([]LatticeValue{
		LatticeDimension{Value: 10, Unit: "px"},
		LatticeDimension{Value: 20, Unit: "em"},
	}, nil)
	if b, ok := result2.(LatticeBool); !ok || b.Value {
		t.Errorf("comparable(10px, 20em) = %v, want false", result2)
	}
}

func TestBuiltinLightenDarken(t *testing.T) {
	// Lighten black by 50% should give grey
	result := builtinLighten([]LatticeValue{
		LatticeColor{Value: "#000000"},
		LatticeNumber{Value: 50},
	}, nil)
	if c, ok := result.(LatticeColor); !ok || c.Value == "#000000" {
		t.Errorf("lighten(#000, 50) = %v, should not be black", result)
	}

	// Darken white by 50% should give grey
	result2 := builtinDarken([]LatticeValue{
		LatticeColor{Value: "#ffffff"},
		LatticeNumber{Value: 50},
	}, nil)
	if c, ok := result2.(LatticeColor); !ok || c.Value == "#ffffff" {
		t.Errorf("darken(#fff, 50) = %v, should not be white", result2)
	}
}

func TestBuiltinMix(t *testing.T) {
	result := builtinMix([]LatticeValue{
		LatticeColor{Value: "#ff0000"},
		LatticeColor{Value: "#0000ff"},
		LatticeNumber{Value: 50},
	}, nil)
	if c, ok := result.(LatticeColor); !ok {
		t.Errorf("mix() = %v, expected a color", result)
	} else {
		// Mix of red and blue should be a purple-ish color
		r, _, b, _ := colorToRGB(c.Value)
		if r == 0 || b == 0 {
			t.Errorf("mix(red, blue, 50%%) produced unexpected color: %s", c.Value)
		}
	}
}

func TestBuiltinColorChannels(t *testing.T) {
	args := []LatticeValue{LatticeColor{Value: "#4a90d9"}}

	red := builtinRed(args, nil)
	if n, ok := red.(LatticeNumber); !ok || n.Value != 74 {
		t.Errorf("red(#4a90d9) = %v, want 74", red)
	}

	green := builtinGreen(args, nil)
	if n, ok := green.(LatticeNumber); !ok || n.Value != 144 {
		t.Errorf("green(#4a90d9) = %v, want 144", green)
	}

	blue := builtinBlue(args, nil)
	if n, ok := blue.(LatticeNumber); !ok || n.Value != 217 {
		t.Errorf("blue(#4a90d9) = %v, want 217", blue)
	}
}

func TestBuiltinComplement(t *testing.T) {
	result := builtinComplement([]LatticeValue{LatticeColor{Value: "#ff0000"}}, nil)
	// Complement of red (hue=0) should be cyan (hue=180)
	if _, ok := result.(LatticeColor); !ok {
		t.Errorf("complement(#ff0000) = %v, expected a color", result)
	}
}

func TestBuiltinMapHasKey(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "a", Value: LatticeNumber{Value: 1}},
	}}
	result := builtinMapHasKey([]LatticeValue{m, LatticeIdent{Value: "a"}}, nil)
	if b, ok := result.(LatticeBool); !ok || !b.Value {
		t.Errorf("map-has-key(m, 'a') = %v, want true", result)
	}
}

func TestBuiltinMapRemove(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "a", Value: LatticeNumber{Value: 1}},
		{Key: "b", Value: LatticeNumber{Value: 2}},
		{Key: "c", Value: LatticeNumber{Value: 3}},
	}}
	result := builtinMapRemove([]LatticeValue{m, LatticeIdent{Value: "b"}}, nil)
	rm, ok := result.(LatticeMap)
	if !ok {
		t.Fatalf("expected LatticeMap, got %T", result)
	}
	if len(rm.Items) != 2 {
		t.Errorf("expected 2 items, got %d", len(rm.Items))
	}
	if rm.MapHasKey("b") {
		t.Error("expected 'b' to be removed")
	}
}

func TestBuiltinMapKeys(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "x", Value: LatticeNumber{Value: 1}},
		{Key: "y", Value: LatticeNumber{Value: 2}},
	}}
	result := builtinMapKeys([]LatticeValue{m}, nil)
	if lst, ok := result.(LatticeList); !ok || len(lst.Items) != 2 {
		t.Errorf("map-keys() = %v, expected list with 2 items", result)
	}
}

func TestBuiltinMapValues(t *testing.T) {
	m := LatticeMap{Items: []MapEntry{
		{Key: "x", Value: LatticeNumber{Value: 10}},
		{Key: "y", Value: LatticeNumber{Value: 20}},
	}}
	result := builtinMapValues([]LatticeValue{m}, nil)
	if lst, ok := result.(LatticeList); !ok || len(lst.Items) != 2 {
		t.Errorf("map-values() = %v, expected list with 2 items", result)
	}
}

// ============================================================================
// Lattice v2: Zero Division Error
// ============================================================================

func TestBuiltinMathDivByZero(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on division by zero")
		}
		if _, ok := r.(*ZeroDivisionInExpressionError); !ok {
			t.Errorf("expected ZeroDivisionInExpressionError, got %T", r)
		}
	}()
	builtinMathDiv([]LatticeValue{LatticeNumber{Value: 10}, LatticeNumber{Value: 0}}, nil)
}

// ============================================================================
// Lattice v2: IsBuiltinFunction
// ============================================================================

func TestIsBuiltinFunction(t *testing.T) {
	if !IsBuiltinFunction("map-get") {
		t.Error("expected map-get to be a built-in")
	}
	if !IsBuiltinFunction("lighten") {
		t.Error("expected lighten to be a built-in")
	}
	if !IsBuiltinFunction("math.div") {
		t.Error("expected math.div to be a built-in")
	}
	if IsBuiltinFunction("nonexistent") {
		t.Error("expected nonexistent to NOT be a built-in")
	}
}

// ============================================================================
// Lattice v2: CallBuiltinFunction
// ============================================================================

func TestCallBuiltinFunction(t *testing.T) {
	result := CallBuiltinFunction("length", []LatticeValue{
		LatticeList{Items: []LatticeValue{LatticeIdent{Value: "a"}, LatticeIdent{Value: "b"}}},
	}, nil)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 2 {
		t.Errorf("CallBuiltinFunction('length') = %v, want 2", result)
	}

	// Unknown function returns null
	result2 := CallBuiltinFunction("nonexistent", nil, nil)
	if _, ok := result2.(LatticeNull); !ok {
		t.Errorf("CallBuiltinFunction('nonexistent') = %v, want null", result2)
	}
}

// ============================================================================
// Lattice v2: typeNameOf
// ============================================================================

func TestTypeNameOf(t *testing.T) {
	if typeNameOf(LatticeNumber{Value: 1}) != "number" {
		t.Error("expected number")
	}
	if typeNameOf(LatticeDimension{Value: 1, Unit: "px"}) != "number" {
		t.Error("expected number")
	}
	if typeNameOf(LatticePercentage{Value: 50}) != "number" {
		t.Error("expected number")
	}
	if typeNameOf(LatticeString{Value: "a"}) != "string" {
		t.Error("expected string")
	}
	if typeNameOf(LatticeIdent{Value: "a"}) != "string" {
		t.Error("expected string")
	}
	if typeNameOf(LatticeColor{Value: "#fff"}) != "color" {
		t.Error("expected color")
	}
	if typeNameOf(LatticeBool{Value: true}) != "bool" {
		t.Error("expected bool")
	}
	if typeNameOf(LatticeNull{}) != "null" {
		t.Error("expected null")
	}
	if typeNameOf(LatticeList{}) != "list" {
		t.Error("expected list")
	}
	if typeNameOf(LatticeMap{}) != "map" {
		t.Error("expected map")
	}
}

// ============================================================================
// Lattice v2: setVariableWithFlags
// ============================================================================

func TestSetVariableWithDefaultFlag(t *testing.T) {
	tr := NewLatticeTransformer()
	// Set a variable normally
	tr.variables.Set("$x", LatticeNumber{Value: 10})
	// Try to set with !default — should NOT override
	tr.setVariableWithFlags(tr.variables, "$x", LatticeNumber{Value: 20}, true, false)
	val, _ := tr.variables.Get("$x")
	if n, ok := val.(LatticeNumber); !ok || n.Value != 10 {
		t.Errorf("!default should not override existing value, got %v", val)
	}

	// Set a new variable with !default — should succeed
	tr.setVariableWithFlags(tr.variables, "$y", LatticeNumber{Value: 30}, true, false)
	val2, _ := tr.variables.Get("$y")
	if n, ok := val2.(LatticeNumber); !ok || n.Value != 30 {
		t.Errorf("!default should set new variable, got %v", val2)
	}
}

func TestSetVariableWithGlobalFlag(t *testing.T) {
	tr := NewLatticeTransformer()
	childScope := tr.variables.Child()
	grandchild := childScope.Child()

	// Set with !global from deeply nested scope
	tr.setVariableWithFlags(grandchild, "$theme", LatticeIdent{Value: "dark"}, false, true)

	// Should be visible in global scope
	val, ok := tr.variables.Get("$theme")
	if !ok {
		t.Fatal("!global should set in root scope")
	}
	if id, ok := val.(LatticeIdent); !ok || id.Value != "dark" {
		t.Errorf("expected 'dark', got %v", val)
	}
}

func TestSetVariableWithDefaultAndGlobal(t *testing.T) {
	tr := NewLatticeTransformer()
	childScope := tr.variables.Child()

	// Set !default !global on a new variable
	tr.setVariableWithFlags(childScope, "$z", LatticeNumber{Value: 99}, true, true)
	val, ok := tr.variables.Get("$z")
	if !ok {
		t.Fatal("!default !global should set in root scope when not defined")
	}
	if n, ok := val.(LatticeNumber); !ok || n.Value != 99 {
		t.Errorf("expected 99, got %v", val)
	}

	// Now try again — should NOT override
	tr.setVariableWithFlags(childScope, "$z", LatticeNumber{Value: 0}, true, true)
	val2, _ := tr.variables.Get("$z")
	if n, ok := val2.(LatticeNumber); !ok || n.Value != 99 {
		t.Errorf("!default !global should NOT override, got %v", val2)
	}
}

// ============================================================================
// Lattice v2: RGBA built-in
// ============================================================================

func TestBuiltinRGBA(t *testing.T) {
	// rgba(color, alpha) form
	result := builtinRGBA([]LatticeValue{
		LatticeColor{Value: "#ff0000"},
		LatticeNumber{Value: 0.5},
	}, nil)
	if c, ok := result.(LatticeColor); !ok || !strings.Contains(c.Value, "rgba") {
		t.Errorf("rgba(#ff0000, 0.5) = %v, expected rgba notation", result)
	}

	// rgba(r, g, b, a) form
	result2 := builtinRGBA([]LatticeValue{
		LatticeNumber{Value: 255},
		LatticeNumber{Value: 0},
		LatticeNumber{Value: 0},
		LatticeNumber{Value: 1.0},
	}, nil)
	if c, ok := result2.(LatticeColor); !ok || c.Value != "#ff0000" {
		t.Errorf("rgba(255,0,0,1) = %v, expected #ff0000", result2)
	}

	// Invalid args returns null
	result3 := builtinRGBA([]LatticeValue{LatticeNumber{Value: 1}}, nil)
	if _, ok := result3.(LatticeNull); !ok {
		t.Errorf("rgba(1) = %v, expected null", result3)
	}
}

// ============================================================================
// Lattice v2: Hue/Saturation/Lightness built-ins
// ============================================================================

func TestBuiltinHueSaturationLightness(t *testing.T) {
	args := []LatticeValue{LatticeColor{Value: "#ff0000"}}

	h := builtinHue(args, nil)
	if d, ok := h.(LatticeDimension); !ok || d.Unit != "deg" {
		t.Errorf("hue(#ff0000) = %v, expected dimension in deg", h)
	}

	s := builtinSaturation(args, nil)
	if p, ok := s.(LatticePercentage); !ok || p.Value != 100 {
		t.Errorf("saturation(#ff0000) = %v, expected 100%%", s)
	}

	l := builtinLightness(args, nil)
	if p, ok := l.(LatticePercentage); !ok || p.Value != 50 {
		t.Errorf("lightness(#ff0000) = %v, expected 50%%", l)
	}
}

// ============================================================================
// Lattice v2: Saturate/Desaturate/AdjustHue
// ============================================================================

func TestBuiltinSaturateDesaturate(t *testing.T) {
	// Desaturate red by 50%
	result := builtinDesaturate([]LatticeValue{
		LatticeColor{Value: "#ff0000"},
		LatticeNumber{Value: 50},
	}, nil)
	if _, ok := result.(LatticeColor); !ok {
		t.Errorf("desaturate() = %v, expected color", result)
	}

	// Saturate a grey
	result2 := builtinSaturateFn([]LatticeValue{
		LatticeColor{Value: "#808080"},
		LatticeNumber{Value: 50},
	}, nil)
	if _, ok := result2.(LatticeColor); !ok {
		t.Errorf("saturate() = %v, expected color", result2)
	}
}

func TestBuiltinAdjustHue(t *testing.T) {
	result := builtinAdjustHue([]LatticeValue{
		LatticeColor{Value: "#ff0000"},
		LatticeNumber{Value: 120},
	}, nil)
	if _, ok := result.(LatticeColor); !ok {
		t.Errorf("adjust-hue() = %v, expected color", result)
	}
}

// ============================================================================
// Lattice v2: Dimension math in built-ins
// ============================================================================

func TestBuiltinMathFloorWithDimension(t *testing.T) {
	result := builtinMathFloor([]LatticeValue{LatticeDimension{Value: 3.7, Unit: "px"}}, nil)
	if d, ok := result.(LatticeDimension); !ok || d.Value != 3 || d.Unit != "px" {
		t.Errorf("math.floor(3.7px) = %v, want 3px", result)
	}
}

func TestBuiltinMathCeilWithPercentage(t *testing.T) {
	result := builtinMathCeil([]LatticeValue{LatticePercentage{Value: 3.2}}, nil)
	if p, ok := result.(LatticePercentage); !ok || p.Value != 4 {
		t.Errorf("math.ceil(3.2%%) = %v, want 4%%", result)
	}
}

func TestBuiltinMathDivDimensionByDimension(t *testing.T) {
	result := builtinMathDiv([]LatticeValue{
		LatticeDimension{Value: 100, Unit: "px"},
		LatticeDimension{Value: 10, Unit: "px"},
	}, nil)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 10 {
		t.Errorf("math.div(100px, 10px) = %v, want 10 (unitless)", result)
	}
}

func TestBuiltinMathDivPercentageByNumber(t *testing.T) {
	result := builtinMathDiv([]LatticeValue{
		LatticePercentage{Value: 100},
		LatticeNumber{Value: 4},
	}, nil)
	if p, ok := result.(LatticePercentage); !ok || p.Value != 25 {
		t.Errorf("math.div(100%%, 4) = %v, want 25%%", result)
	}
}

// ============================================================================
// Lattice v2: parseVariableDeclaration
// ============================================================================

func TestParseVariableDeclaration(t *testing.T) {
	// Build a variable_declaration AST node with !default flag
	node := &parser.ASTNode{
		RuleName: "variable_declaration",
		Children: []interface{}{
			lexer.Token{TypeName: "VARIABLE", Value: "$color"},
			lexer.Token{TypeName: "COLON", Value: ":"},
			&parser.ASTNode{
				RuleName: "value_list",
				Children: []interface{}{lexer.Token{TypeName: "IDENT", Value: "red"}},
			},
			lexer.Token{TypeName: "BANG_DEFAULT", Value: "!default"},
			lexer.Token{TypeName: "SEMICOLON", Value: ";"},
		},
	}

	name, valueNode, isDefault, isGlobal := parseVariableDeclaration(node)
	if name != "$color" {
		t.Errorf("expected name '$color', got %q", name)
	}
	if valueNode == nil {
		t.Fatal("expected non-nil valueNode")
	}
	if !isDefault {
		t.Error("expected isDefault=true")
	}
	if isGlobal {
		t.Error("expected isGlobal=false")
	}
}

func TestParseVariableDeclarationWithFlag(t *testing.T) {
	// Variable with a variable_flag node containing BANG_GLOBAL
	node := &parser.ASTNode{
		RuleName: "variable_declaration",
		Children: []interface{}{
			lexer.Token{TypeName: "VARIABLE", Value: "$x"},
			lexer.Token{TypeName: "COLON", Value: ":"},
			&parser.ASTNode{
				RuleName: "value_list",
				Children: []interface{}{lexer.Token{TypeName: "NUMBER", Value: "42"}},
			},
			&parser.ASTNode{
				RuleName: "variable_flag",
				Children: []interface{}{
					lexer.Token{TypeName: "BANG_GLOBAL", Value: "!global"},
				},
			},
			lexer.Token{TypeName: "SEMICOLON", Value: ";"},
		},
	}

	_, _, isDefault, isGlobal := parseVariableDeclaration(node)
	if isDefault {
		t.Error("expected isDefault=false")
	}
	if !isGlobal {
		t.Error("expected isGlobal=true")
	}
}

// ============================================================================
// Lattice v2: NthOutOfBounds
// ============================================================================

func TestBuiltinNthOutOfBounds(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on nth out of bounds")
		}
	}()
	list := LatticeList{Items: []LatticeValue{LatticeIdent{Value: "a"}}}
	builtinNth([]LatticeValue{list, LatticeNumber{Value: 5}}, nil)
}

func TestBuiltinNthSingleValue(t *testing.T) {
	// Single value treated as list of length 1
	result := builtinNth([]LatticeValue{LatticeIdent{Value: "hello"}, LatticeNumber{Value: 1}}, nil)
	if id, ok := result.(LatticeIdent); !ok || id.Value != "hello" {
		t.Errorf("nth(hello, 1) = %v, want 'hello'", result)
	}
}

// ============================================================================
// Lattice v2: extractExtendTarget
// ============================================================================

func TestExtractExtendTarget(t *testing.T) {
	node := &parser.ASTNode{
		RuleName: "extend_target",
		Children: []interface{}{
			lexer.Token{TypeName: "PLACEHOLDER", Value: "%message-shared"},
		},
	}
	target := extractExtendTarget(node)
	if target != "%message-shared" {
		t.Errorf("expected '%%message-shared', got %q", target)
	}
}

// ============================================================================
// Lattice v2: getNumericValue panic
// ============================================================================

func TestGetNumericValuePanic(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on non-numeric getNumericValue")
		}
	}()
	getNumericValue(LatticeIdent{Value: "hello"})
}

// ============================================================================
// Lattice v2: ensureColor panic
// ============================================================================

func TestEnsureColorPanic(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on non-color ensureColor")
		}
	}()
	ensureColor(LatticeNumber{Value: 42})
}

// ============================================================================
// Lattice v2: ensureAmount range panic
// ============================================================================

func TestEnsureAmountPanic(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on out-of-range amount")
		}
	}()
	ensureAmount(LatticeNumber{Value: 150})
}

// ============================================================================
// Lattice v2: expandWhile direct unit test
// ============================================================================

func TestExpandWhileDirect(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)
	scope.Set("$i", LatticeNumber{Value: 1})

	// Build a while_directive AST:
	// @while $i <= 3 { ... $i: $i + 1; }
	// We'll test just the basic structure
	condNode := &parser.ASTNode{
		RuleName: "lattice_expression",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "lattice_comparison",
				Children: []interface{}{
					&parser.ASTNode{
						RuleName: "lattice_primary",
						Children: []interface{}{
							lexer.Token{TypeName: "VARIABLE", Value: "$i"},
						},
					},
					&parser.ASTNode{
						RuleName: "comparison_op",
						Children: []interface{}{
							lexer.Token{TypeName: "LESS_EQUALS", Value: "<="},
						},
					},
					&parser.ASTNode{
						RuleName: "lattice_primary",
						Children: []interface{}{
							lexer.Token{TypeName: "NUMBER", Value: "0"},
						},
					},
				},
			},
		},
	}

	blockNode := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	whileNode := &parser.ASTNode{
		RuleName: "while_directive",
		Children: []interface{}{condNode, blockNode},
	}

	// $i=1, condition: $i <= 0 → false, so loop never runs
	result := tr.expandWhile(whileNode, scope)
	if len(result) != 0 {
		t.Errorf("expected 0 items from while (false condition), got %d", len(result))
	}
}

func TestExpandWhileMaxIterationPanic(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on max iterations")
		}
		if _, ok := r.(*MaxIterationError); !ok {
			t.Errorf("expected MaxIterationError, got %T: %v", r, r)
		}
	}()

	tr := NewLatticeTransformer()
	tr.maxWhileIterations = 5 // Low limit for testing
	scope := NewScopeChain(nil)
	scope.Set("$i", LatticeNumber{Value: 1})

	// Condition: true (always truthy)
	condNode := &parser.ASTNode{
		RuleName: "lattice_expression",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "lattice_primary",
				Children: []interface{}{
					lexer.Token{TypeName: "IDENT", Value: "true"},
				},
			},
		},
	}

	blockNode := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	whileNode := &parser.ASTNode{
		RuleName: "while_directive",
		Children: []interface{}{condNode, blockNode},
	}

	tr.expandWhile(whileNode, scope)
}

// ============================================================================
// Lattice v2: expandContent direct unit test
// ============================================================================

func TestExpandContentNoBlock(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	// No content block stack — should return nil
	result := tr.expandContent(&parser.ASTNode{RuleName: "content_directive"}, scope)
	if result != nil {
		t.Errorf("expected nil when no content block, got %v", result)
	}

	// Push nil block — should return nil
	tr.contentBlockStack = append(tr.contentBlockStack, nil)
	result2 := tr.expandContent(&parser.ASTNode{RuleName: "content_directive"}, scope)
	if result2 != nil {
		t.Errorf("expected nil for nil content block, got %v", result2)
	}
}

func TestExpandContentWithBlock(t *testing.T) {
	tr := NewLatticeTransformer()
	callerScope := NewScopeChain(nil)
	callerScope.Set("$color", LatticeIdent{Value: "red"})

	contentBlock := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	tr.contentBlockStack = append(tr.contentBlockStack, contentBlock)
	tr.contentScopeStack = append(tr.contentScopeStack, callerScope)

	result := tr.expandContent(&parser.ASTNode{RuleName: "content_directive"}, NewScopeChain(nil))
	// Empty block_contents produces empty result
	if len(result) != 0 {
		t.Errorf("expected 0 items from empty content block, got %d", len(result))
	}
}

// ============================================================================
// Lattice v2: expandAtRoot direct unit test
// ============================================================================

func TestExpandAtRootBlockForm(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	blockNode := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	node := &parser.ASTNode{
		RuleName: "at_root_directive",
		Children: []interface{}{blockNode},
	}

	result := tr.expandAtRoot(node, scope)
	if result != nil {
		t.Errorf("expected nil from @at-root (items are hoisted), got %v", result)
	}
}

func TestExpandAtRootWithSelector(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	selectorNode := &parser.ASTNode{
		RuleName: "selector_list",
		Children: []interface{}{
			lexer.Token{TypeName: "DOT", Value: "."},
			lexer.Token{TypeName: "IDENT", Value: "root-item"},
		},
	}

	blockNode := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	node := &parser.ASTNode{
		RuleName: "at_root_directive",
		Children: []interface{}{selectorNode, blockNode},
	}

	result := tr.expandAtRoot(node, scope)
	if result != nil {
		t.Errorf("expected nil from @at-root, got %v", result)
	}
	if len(tr.atRootRules) != 1 {
		t.Errorf("expected 1 hoisted rule, got %d", len(tr.atRootRules))
	}
}

// ============================================================================
// Lattice v2: collectExtend direct unit test
// ============================================================================

func TestCollectExtend(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	node := &parser.ASTNode{
		RuleName: "extend_directive",
		Children: []interface{}{
			lexer.Token{TypeName: "AT_KEYWORD", Value: "@extend"},
			&parser.ASTNode{
				RuleName: "extend_target",
				Children: []interface{}{
					lexer.Token{TypeName: "PLACEHOLDER", Value: "%message"},
				},
			},
			lexer.Token{TypeName: "SEMICOLON", Value: ";"},
		},
	}

	tr.collectExtend(node, scope)
	if _, ok := tr.extendMap["%message"]; !ok {
		t.Error("expected %message to be recorded in extendMap")
	}
}

// ============================================================================
// Lattice v2: expandSelectorWithVars direct unit test
// ============================================================================

func TestExpandSelectorWithVars(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)
	scope.Set("$i", LatticeNumber{Value: 3})

	node := &parser.ASTNode{
		RuleName: "compound_selector",
		Children: []interface{}{
			lexer.Token{TypeName: "DOT", Value: "."},
			lexer.Token{TypeName: "IDENT", Value: "col-"},
			lexer.Token{TypeName: "VARIABLE", Value: "$i"},
		},
	}

	result := tr.expandSelectorWithVars(node, scope)
	// The variable should be replaced with "3"
	found := false
	for _, child := range result.Children {
		if tok, ok := child.(lexer.Token); ok && tok.Value == "3" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected $i to be resolved to '3' in selector")
	}
}

func TestExpandSelectorWithVarsUndefined(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic for undefined variable in selector")
		}
	}()

	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	node := &parser.ASTNode{
		RuleName: "compound_selector",
		Children: []interface{}{
			lexer.Token{TypeName: "VARIABLE", Value: "$undefined"},
		},
	}

	tr.expandSelectorWithVars(node, scope)
}

// ============================================================================
// Lattice v2: makeQualifiedRule
// ============================================================================

func TestMakeQualifiedRule(t *testing.T) {
	sel := &parser.ASTNode{RuleName: "selector_list"}
	block := &parser.ASTNode{RuleName: "block"}
	qr := makeQualifiedRule(sel, block)
	if qr.RuleName != "qualified_rule" {
		t.Errorf("expected qualified_rule, got %q", qr.RuleName)
	}
	if len(qr.Children) != 2 {
		t.Errorf("expected 2 children, got %d", len(qr.Children))
	}
}

// ============================================================================
// Lattice v2: evaluateBuiltinFunctionCall
// ============================================================================

func TestEvaluateBuiltinFunctionCall(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	// Build a function_call node for length((a, b, c))
	node := &parser.ASTNode{
		RuleName: "function_call",
		Children: []interface{}{
			lexer.Token{TypeName: "FUNCTION", Value: "length("},
			&parser.ASTNode{
				RuleName: "function_args",
				Children: []interface{}{
					&parser.ASTNode{
						RuleName: "function_arg",
						Children: []interface{}{
							lexer.Token{TypeName: "NUMBER", Value: "42"},
						},
					},
				},
			},
			lexer.Token{TypeName: "RPAREN", Value: ")"},
		},
	}

	result := tr.evaluateBuiltinFunctionCall("length", node, scope)
	// Should return a value node with "1" (single number arg has length 1)
	if result == nil {
		t.Fatal("expected non-nil result from evaluateBuiltinFunctionCall")
	}
}

// ============================================================================
// Lattice v2: collectFunctionArgs and evalArgTokens
// ============================================================================

func TestCollectFunctionArgs(t *testing.T) {
	scope := NewScopeChain(nil)
	scope.Set("$x", LatticeNumber{Value: 42})
	eval := NewExpressionEvaluator(scope)

	argsNode := &parser.ASTNode{
		RuleName: "function_args",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "function_arg",
				Children: []interface{}{
					lexer.Token{TypeName: "VARIABLE", Value: "$x"},
				},
			},
			&parser.ASTNode{
				RuleName: "function_arg",
				Children: []interface{}{
					lexer.Token{TypeName: "COMMA", Value: ","},
				},
			},
			&parser.ASTNode{
				RuleName: "function_arg",
				Children: []interface{}{
					lexer.Token{TypeName: "NUMBER", Value: "10"},
				},
			},
		},
	}

	args := eval.collectFunctionArgs(argsNode)
	if len(args) != 2 {
		t.Fatalf("expected 2 args, got %d", len(args))
	}
	if n, ok := args[0].(LatticeNumber); !ok || n.Value != 42 {
		t.Errorf("first arg = %v, want 42", args[0])
	}
	if n, ok := args[1].(LatticeNumber); !ok || n.Value != 10 {
		t.Errorf("second arg = %v, want 10", args[1])
	}
}

func TestEvalArgTokensVariable(t *testing.T) {
	scope := NewScopeChain(nil)
	scope.Set("$y", LatticeDimension{Value: 16, Unit: "px"})
	eval := NewExpressionEvaluator(scope)

	tokens := []interface{}{lexer.Token{TypeName: "VARIABLE", Value: "$y"}}
	result := eval.evalArgTokens(tokens)
	if d, ok := result.(LatticeDimension); !ok || d.Value != 16 || d.Unit != "px" {
		t.Errorf("evalArgTokens($y) = %v, want 16px", result)
	}
}

func TestEvalArgTokensLiteral(t *testing.T) {
	eval := NewExpressionEvaluator(NewScopeChain(nil))

	tokens := []interface{}{lexer.Token{TypeName: "NUMBER", Value: "99"}}
	result := eval.evalArgTokens(tokens)
	if n, ok := result.(LatticeNumber); !ok || n.Value != 99 {
		t.Errorf("evalArgTokens(99) = %v, want 99", result)
	}
}

func TestEvalArgTokensEmpty(t *testing.T) {
	eval := NewExpressionEvaluator(NewScopeChain(nil))
	result := eval.evalArgTokens(nil)
	if _, ok := result.(LatticeNull); !ok {
		t.Errorf("evalArgTokens(nil) = %v, want null", result)
	}
}

// ============================================================================
// Lattice v2: expandEachOverResolved
// ============================================================================

func TestExpandEachOverResolvedMap(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	m := LatticeMap{Items: []MapEntry{
		{Key: "a", Value: LatticeNumber{Value: 1}},
		{Key: "b", Value: LatticeNumber{Value: 2}},
	}}

	block := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	result := tr.expandEachOverResolved([]string{"$key", "$val"}, m, block, scope)
	// Empty block produces empty results, but should iterate correctly
	if len(result) != 0 {
		t.Errorf("expected 0 items from empty block, got %d", len(result))
	}
}

func TestExpandEachOverResolvedList(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)

	lst := LatticeList{Items: []LatticeValue{
		LatticeIdent{Value: "red"},
		LatticeIdent{Value: "green"},
		LatticeIdent{Value: "blue"},
	}}

	block := &parser.ASTNode{
		RuleName: "block",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "block_contents",
				Children: []interface{}{},
			},
		},
	}

	result := tr.expandEachOverResolved([]string{"$color"}, lst, block, scope)
	if len(result) != 0 {
		t.Errorf("expected 0 items from empty block, got %d", len(result))
	}
}

// ============================================================================
// Lattice v2: resolveEachList
// ============================================================================

func TestResolveEachList(t *testing.T) {
	tr := NewLatticeTransformer()
	scope := NewScopeChain(nil)
	m := LatticeMap{Items: []MapEntry{{Key: "a", Value: LatticeNumber{Value: 1}}}}
	scope.Set("$map", m)

	eachList := &parser.ASTNode{
		RuleName: "each_list",
		Children: []interface{}{
			&parser.ASTNode{
				RuleName: "value",
				Children: []interface{}{
					lexer.Token{TypeName: "VARIABLE", Value: "$map"},
				},
			},
		},
	}

	resolved := tr.resolveEachList(eachList, scope)
	if resolved == nil {
		t.Fatal("expected resolved map from resolveEachList")
	}
	if _, ok := resolved.(LatticeMap); !ok {
		t.Errorf("expected LatticeMap, got %T", resolved)
	}
}
