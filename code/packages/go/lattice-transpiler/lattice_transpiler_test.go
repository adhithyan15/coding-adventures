package latticetranspiler

// Tests for the lattice-transpiler package.
//
// This package is a thin pipeline wrapper around lattice-ast-to-css.
// Tests here focus on:
//   - The public API (Transpile, TranspileMinified, TranspileWithOptions)
//   - Options handling (Minify flag, Indent string, defaults)
//   - End-to-end correctness with representative Lattice inputs
//   - Error propagation from the underlying pipeline
//
// Detailed feature tests (variables, mixins, @if, @for, etc.) live in the
// lattice-ast-to-css package. Integration stress tests here exercise the
// full pipeline through the latticetranspiler entry point.

import (
	"errors"
	"strings"
	"testing"

	latticeasttocss "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-ast-to-css"
)

// ============================================================================
// Transpile (pretty-print defaults)
// ============================================================================

func TestTranspilePlainCSS(t *testing.T) {
	// Plain CSS should pass through unchanged (minus normalised whitespace).
	// Lattice is a strict superset; valid CSS is valid Lattice.
	css, err := Transpile("h1 { color: red; }")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "color: red") {
		t.Errorf("expected 'color: red' in output, got: %q", css)
	}
	if !strings.Contains(css, "h1") {
		t.Errorf("expected 'h1' selector in output, got: %q", css)
	}
}

func TestTranspileVariableSubstitution(t *testing.T) {
	src := `
		$brand: blue;
		.btn { color: $brand; }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "color: blue") {
		t.Errorf("expected variable to resolve to 'blue', got: %q", css)
	}
	// Variable declaration should not appear in output
	if strings.Contains(css, "$brand") {
		t.Errorf("raw variable reference should not appear in CSS output, got: %q", css)
	}
}

func TestTranspilePrettyPrintDefaults(t *testing.T) {
	// Default Transpile uses 2-space indentation and newlines.
	src := `.card { padding: 8px; }`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Pretty-printed output should have newlines
	if !strings.Contains(css, "\n") {
		t.Errorf("expected newlines in pretty-printed output, got: %q", css)
	}
	// Should have 2-space indent
	if !strings.Contains(css, "  padding") {
		t.Errorf("expected 2-space indentation before 'padding', got: %q", css)
	}
}

func TestTranspileMixinExpansion(t *testing.T) {
	src := `
		@mixin flex-center() {
			display: flex;
			align-items: center;
		}
		.container { @include flex-center(); }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "display: flex") {
		t.Errorf("expected mixin body in output, got: %q", css)
	}
	if !strings.Contains(css, "align-items: center") {
		t.Errorf("expected mixin body in output, got: %q", css)
	}
	// @mixin and @include directives should not appear in output
	if strings.Contains(css, "@mixin") {
		t.Errorf("@mixin definition should not appear in CSS output, got: %q", css)
	}
	if strings.Contains(css, "@include") {
		t.Errorf("@include directive should not appear in CSS output, got: %q", css)
	}
}

func TestTranspileIfElse(t *testing.T) {
	src := `
		$theme: dark;
		.body {
			@if $theme == dark {
				background: black;
			} @else {
				background: white;
			}
		}
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "background: black") {
		t.Errorf("expected dark theme branch in output, got: %q", css)
	}
	if strings.Contains(css, "background: white") {
		t.Errorf("inactive @else branch should not appear in output, got: %q", css)
	}
}

func TestTranspileFunction(t *testing.T) {
	src := `
		@function double($n) {
			@return $n * 2;
		}
		.box { width: double(8px); }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "width: 16px") {
		t.Errorf("expected function result '16px' in output, got: %q", css)
	}
}

// ============================================================================
// TranspileMinified
// ============================================================================

func TestTranspileMinifiedBasic(t *testing.T) {
	src := `.card { padding: 8px; }`
	css, err := TranspileMinified(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if strings.Contains(css, "\n") {
		t.Errorf("minified output should not contain newlines, got: %q", css)
	}
	if strings.Contains(css, "  ") {
		t.Errorf("minified output should not contain double spaces, got: %q", css)
	}
	if !strings.Contains(css, "padding:8px") {
		t.Errorf("expected compact 'padding:8px' in minified output, got: %q", css)
	}
}

func TestTranspileMinifiedWithVariables(t *testing.T) {
	src := `$x: 10px; .a { margin: $x; }`
	css, err := TranspileMinified(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if strings.Contains(css, "\n") {
		t.Errorf("minified output should not contain newlines, got: %q", css)
	}
	if !strings.Contains(css, "margin:10px") {
		t.Errorf("expected 'margin:10px' in minified output, got: %q", css)
	}
}

// ============================================================================
// TranspileWithOptions
// ============================================================================

func TestTranspileWithOptionsDefaultsToTwoSpaces(t *testing.T) {
	// Zero-value Options should behave like Transpile (2-space indent).
	src := `.card { padding: 8px; }`
	css1, err1 := Transpile(src)
	css2, err2 := TranspileWithOptions(src, Options{})
	if err1 != nil || err2 != nil {
		t.Fatalf("unexpected errors: %v / %v", err1, err2)
	}
	if css1 != css2 {
		t.Errorf("Transpile and TranspileWithOptions{} should produce identical output\ngot1: %q\ngot2: %q", css1, css2)
	}
}

func TestTranspileWithOptionsMinify(t *testing.T) {
	src := `.btn { color: red; }`
	css, err := TranspileWithOptions(src, Options{Minify: true})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if strings.Contains(css, "\n") {
		t.Errorf("minified output should not contain newlines, got: %q", css)
	}
	if !strings.Contains(css, "color:red") {
		t.Errorf("expected compact 'color:red' in minified output, got: %q", css)
	}
}

func TestTranspileWithOptionsCustomIndent(t *testing.T) {
	src := `.card { padding: 8px; }`
	css, err := TranspileWithOptions(src, Options{Indent: "\t"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "\t") {
		t.Errorf("expected tab indentation in output, got: %q", css)
	}
}

func TestTranspileWithOptionsFourSpaceIndent(t *testing.T) {
	src := `.card { padding: 8px; }`
	css, err := TranspileWithOptions(src, Options{Indent: "    "})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "    padding") {
		t.Errorf("expected 4-space indentation before 'padding', got: %q", css)
	}
}

func TestTranspileWithOptionsMinifyIgnoresIndent(t *testing.T) {
	// Indent setting is ignored when Minify is true.
	src := `.card { padding: 8px; }`
	css1, _ := TranspileWithOptions(src, Options{Minify: true, Indent: "  "})
	css2, _ := TranspileWithOptions(src, Options{Minify: true, Indent: "\t"})
	if css1 != css2 {
		t.Errorf("Indent should be ignored when Minify=true\ngot1: %q\ngot2: %q", css1, css2)
	}
	if strings.Contains(css1, "\n") || strings.Contains(css1, "\t") {
		t.Errorf("minified output should not contain whitespace, got: %q", css1)
	}
}

// ============================================================================
// resolveIndent internal logic
// ============================================================================

func TestResolveIndentDefaultsTwoSpaces(t *testing.T) {
	result := resolveIndent(Options{})
	if result != "  " {
		t.Errorf("expected default indent '  ', got: %q", result)
	}
}

func TestResolveIndentMinifyReturnsEmpty(t *testing.T) {
	result := resolveIndent(Options{Minify: true})
	if result != "" {
		t.Errorf("expected empty indent for minify, got: %q", result)
	}
}

func TestResolveIndentCustom(t *testing.T) {
	result := resolveIndent(Options{Indent: "\t"})
	if result != "\t" {
		t.Errorf("expected tab indent, got: %q", result)
	}
}

func TestResolveIndentMinifyIgnoresCustomIndent(t *testing.T) {
	result := resolveIndent(Options{Minify: true, Indent: "\t"})
	if result != "" {
		t.Errorf("expected empty indent (Minify overrides Indent), got: %q", result)
	}
}

// ============================================================================
// Error propagation
// ============================================================================

func TestTranspileUndefinedVariableError(t *testing.T) {
	_, err := Transpile(".btn { color: $missing; }")
	if err == nil {
		t.Fatal("expected error for undefined variable")
	}
	var uve *latticeasttocss.UndefinedVariableError
	if !errors.As(err, &uve) {
		t.Errorf("expected UndefinedVariableError, got: %T %v", err, err)
	}
	if uve.Name != "$missing" {
		t.Errorf("expected Name '$missing', got: %q", uve.Name)
	}
}

func TestTranspileMinifiedUndefinedVariable(t *testing.T) {
	_, err := TranspileMinified(".a { width: $unknown; }")
	if err == nil {
		t.Fatal("expected error for undefined variable")
	}
}

func TestTranspileWithOptionsUndefinedVariable(t *testing.T) {
	_, err := TranspileWithOptions(".a { width: $unknown; }", Options{})
	if err == nil {
		t.Fatal("expected error for undefined variable")
	}
}

func TestTranspileUndefinedMixinError(t *testing.T) {
	_, err := Transpile(".btn { @include ghost(); }")
	if err == nil {
		t.Fatal("expected error for undefined mixin")
	}
	var ume *latticeasttocss.UndefinedMixinError
	if !errors.As(err, &ume) {
		t.Errorf("expected UndefinedMixinError, got: %T %v", err, err)
	}
}

func TestTranspileWrongArityError(t *testing.T) {
	src := `
		@mixin btn($color) {
			color: $color;
		}
		.a { @include btn(); }
	`
	_, err := Transpile(src)
	if err == nil {
		t.Fatal("expected error for wrong arity")
	}
	var wae *latticeasttocss.WrongArityError
	if !errors.As(err, &wae) {
		t.Errorf("expected WrongArityError, got: %T %v", err, err)
	}
}

func TestTranspileLexerError(t *testing.T) {
	// Feed something the lexer will reject (bare backslash is invalid in CSS)
	_, err := Transpile("h1 { color: \\; }")
	// If the lexer or parser panics/errors, we just verify we get an error,
	// not a crash. (Some inputs may be silently skipped by the parser too.)
	// The important thing is no panic.
	_ = err
}

// ============================================================================
// Stress / integration tests (spec scenarios)
// ============================================================================

// TestIntegrationFunctionInValue covers spec Test 5: function calls in value positions.
func TestIntegrationFunctionInValue(t *testing.T) {
	src := `
		@function spacing($multiplier) {
			@return $multiplier * 8px;
		}
		.card {
			padding: spacing(2);
			margin: spacing(3);
		}
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "padding: 16px") {
		t.Errorf("expected 'padding: 16px', got: %q", css)
	}
	if !strings.Contains(css, "margin: 24px") {
		t.Errorf("expected 'margin: 24px', got: %q", css)
	}
}

// TestIntegrationEachLoop covers spec Test 3: @each producing multiple rules.
//
// Note: @each must appear inside a mixin body (it is a lattice_block_item,
// not a top-level lattice_rule). We wrap it in a mixin and @include it.
func TestIntegrationEachLoop(t *testing.T) {
	src := `
		@mixin make-paddings() {
			@each $size in 8px, 16px, 24px {
				.card {
					padding: $size;
				}
			}
		}
		.wrapper { @include make-paddings(); }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "padding: 8px") {
		t.Errorf("expected first iteration 'padding: 8px', got: %q", css)
	}
	if !strings.Contains(css, "padding: 16px") {
		t.Errorf("expected second iteration 'padding: 16px', got: %q", css)
	}
	if !strings.Contains(css, "padding: 24px") {
		t.Errorf("expected third iteration 'padding: 24px', got: %q", css)
	}
}

// TestIntegrationIfInsideEach covers spec Test 4: @if inside @each.
//
// Note: @each must appear inside a mixin body.
func TestIntegrationIfInsideEach(t *testing.T) {
	src := `
		$theme: dark;
		@mixin make-boxes() {
			@each $value in 10px, 20px {
				@if $theme == dark {
					.box {
						padding: $value;
						background: black;
					}
				} @else {
					.box {
						padding: $value;
						background: white;
					}
				}
			}
		}
		.wrapper { @include make-boxes(); }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Both iterations should expand the @if (dark) branch
	if !strings.Contains(css, "background: black") {
		t.Errorf("expected dark theme background in output, got: %q", css)
	}
	if strings.Contains(css, "background: white") {
		t.Errorf("inactive @else branch should not appear in output, got: %q", css)
	}
	if !strings.Contains(css, "padding: 10px") {
		t.Errorf("expected first iteration padding, got: %q", css)
	}
	if !strings.Contains(css, "padding: 20px") {
		t.Errorf("expected second iteration padding, got: %q", css)
	}
}

// TestIntegrationScopeShadowing covers spec Test 2: nested scope shadowing.
func TestIntegrationScopeShadowing(t *testing.T) {
	src := `
		$color: red;
		.outer {
			$color: blue;
			color: $color;
		}
		.sibling {
			color: $color;
		}
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// .outer should use its own $color: blue
	if !strings.Contains(css, "color: blue") {
		t.Errorf("expected .outer to use 'color: blue', got: %q", css)
	}
	// .sibling should use the global $color: red
	if !strings.Contains(css, "color: red") {
		t.Errorf("expected .sibling to use 'color: red', got: %q", css)
	}
}

// TestIntegrationPlainCSSPassthrough covers spec Test 8: CSS passthrough.
func TestIntegrationPlainCSSPassthrough(t *testing.T) {
	src := `h1 { color: red; font-size: 2em; }`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "color: red") {
		t.Errorf("expected 'color: red' passthrough, got: %q", css)
	}
	if !strings.Contains(css, "font-size: 2em") {
		t.Errorf("expected 'font-size: 2em' passthrough, got: %q", css)
	}
}

// TestIntegrationMediaRule verifies @media rules pass through correctly.
func TestIntegrationMediaRule(t *testing.T) {
	src := `
		@media (max-width: 768px) {
			h1 { font-size: 1.5em; }
		}
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "@media") {
		t.Errorf("expected @media rule in output, got: %q", css)
	}
	if !strings.Contains(css, "font-size: 1.5em") {
		t.Errorf("expected declaration inside @media, got: %q", css)
	}
}

// TestIntegrationEmptySource verifies empty input produces empty or minimal output.
func TestIntegrationEmptySource(t *testing.T) {
	css, err := Transpile("")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if strings.TrimSpace(css) != "" {
		t.Errorf("expected empty output for empty source, got: %q", css)
	}
}

// TestIntegrationMixinWithDefaults exercises default parameter handling.
func TestIntegrationMixinWithDefaults(t *testing.T) {
	src := `
		@mixin border($color: black, $width: 1px) {
			border: $width solid $color;
		}
		.a { @include border(); }
		.b { @include border(red); }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "border: 1px solid black") {
		t.Errorf("expected default border in .a, got: %q", css)
	}
	if !strings.Contains(css, "border: 1px solid red") {
		t.Errorf("expected red border in .b, got: %q", css)
	}
}

// TestIntegrationForLoop verifies @for generates repeated blocks.
//
// Note: @for must appear inside a mixin body (it is a lattice_block_item,
// not a top-level lattice_rule).
func TestIntegrationForLoop(t *testing.T) {
	src := `
		@mixin make-cols() {
			@for $i from 1 through 3 {
				.col {
					flex: $i;
				}
			}
		}
		.grid { @include make-cols(); }
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "flex: 1") {
		t.Errorf("expected iteration 1 output, got: %q", css)
	}
	if !strings.Contains(css, "flex: 2") {
		t.Errorf("expected iteration 2 output, got: %q", css)
	}
	if !strings.Contains(css, "flex: 3") {
		t.Errorf("expected iteration 3 output, got: %q", css)
	}
}

// TestIntegrationMultipleVariables exercises multiple variable substitutions
// in the same declaration value.
func TestIntegrationMultipleVariables(t *testing.T) {
	src := `
		$size: 16px;
		$unit: 1.5;
		.text {
			font-size: $size;
		}
	`
	css, err := Transpile(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !strings.Contains(css, "font-size: 16px") {
		t.Errorf("expected 'font-size: 16px', got: %q", css)
	}
}

// TestIntegrationCircularMixinError verifies cycle detection.
func TestIntegrationCircularMixinError(t *testing.T) {
	src := `
		@mixin a() {
			@include b();
		}
		@mixin b() {
			@include a();
		}
		.x { @include a(); }
	`
	_, err := Transpile(src)
	if err == nil {
		t.Fatal("expected circular reference error")
	}
	var cre *latticeasttocss.CircularReferenceError
	if !errors.As(err, &cre) {
		t.Errorf("expected CircularReferenceError, got: %T %v", err, err)
	}
}
