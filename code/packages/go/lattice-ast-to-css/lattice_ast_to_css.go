// Package latticeasttocss is a three-pass compiler from Lattice AST to clean
// CSS AST, plus a CSS emitter that serializes the result to a string.
//
// # What Is Lattice?
//
// Lattice is a CSS superset, similar to Sass or SCSS. It adds:
//
//   - Variables:   $primary: #4a90d9;  → replaced at compile time
//   - Mixins:      @mixin button($bg) { ... }  @include button(red);
//   - Functions:   @function double($n) { @return $n * 2; }
//   - Control:     @if, @else, @for, @each
//   - Modules:     @use "tokens";
//
// Because Lattice compiles to CSS (there is no runtime), all Lattice
// constructs are evaluated at compile time. The output is plain CSS that
// any browser can understand.
//
// # Pipeline
//
//	Lattice source text
//	  ↓ lattice-lexer   — tokenize
//	  ↓ lattice-parser  — parse into Lattice AST
//	  ↓ this package    — compile Lattice AST → CSS AST → CSS text
//	CSS output text
//
// # Public API
//
// The simplest entry point is TranspileLatticeFull:
//
//	css, err := latticeasttocss.TranspileLatticeFull(source, false, "  ")
//
// For step-by-step control:
//
//	transformer := latticeasttocss.NewLatticeTransformer()
//	cssAST, err := transformer.Transform(latticeAST)
//	emitter := latticeasttocss.NewCSSEmitter(false, "  ")
//	css := emitter.Emit(cssAST)
//
// # Error Handling
//
// All errors satisfy the error interface and embed LatticeError. Use
// errors.As to extract structured error info:
//
//	css, err := TranspileLatticeFull(source, false, "  ")
//	var uve *UndefinedVariableError
//	if errors.As(err, &uve) {
//	    fmt.Printf("Undefined variable %s at line %d\n", uve.Name, uve.Line)
//	}
package latticeasttocss

import (
	"fmt"
	"runtime/debug"

	latticelexer "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-lexer"
	latticeparser "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// Top-Level Transpiler
// ============================================================================

// TranspileLatticeFull compiles a Lattice source string to CSS text.
//
// This is the all-in-one function that runs the full pipeline:
// tokenize → parse → transform → emit.
//
// Parameters:
//
//	source   — Lattice CSS source text (e.g., read from a .lattice file)
//	minify   — true for compact output, false for pretty-printed output
//	indent   — indentation string per level (e.g., "  " or "\t"); ignored when minify is true
//
// Returns the compiled CSS string, or an error if the source is invalid.
//
// Example:
//
//	css, err := TranspileLatticeFull(`
//	    $color: red;
//	    .btn { color: $color; }
//	`, false, "  ")
//	// css = ".btn {\n  color: red;\n}\n"
func TranspileLatticeFull(source string, minify bool, indent string) (css string, err error) {
	// Recover from any unexpected panics in the compiler (e.g., parser bugs).
	// Return a descriptive error rather than crashing the caller.
	defer func() {
		if r := recover(); r != nil {
			// If it's a LatticeError subtype, wrap it
			if le, ok := r.(error); ok {
				err = le
				return
			}
			err = fmt.Errorf("internal compiler error: %v\n%s", r, debug.Stack())
		}
	}()

	// Step 1: Tokenize
	tokens, lexErr := latticelexer.TokenizeLatticeLexer(source)
	if lexErr != nil {
		return "", fmt.Errorf("lexer error: %w", lexErr)
	}
	_ = tokens // tokens are used by the parser internally

	// Step 2: Parse
	ast, parseErr := latticeparser.ParseLattice(source)
	if parseErr != nil {
		return "", fmt.Errorf("parse error: %w", parseErr)
	}

	// Step 3: Transform (Lattice AST → CSS AST)
	transformer := NewLatticeTransformer()
	cssAST, transformErr := transformer.Transform(ast)
	if transformErr != nil {
		return "", transformErr
	}

	// Step 4: Emit (CSS AST → CSS text)
	emitter := NewCSSEmitter(minify, indent)
	css = emitter.Emit(cssAST)

	return css, nil
}

// TranspileLattice compiles Lattice source to pretty-printed CSS with 2-space indentation.
//
// This is a convenience wrapper around TranspileLatticeFull with sensible defaults.
//
// Example:
//
//	css, err := TranspileLattice("$x: 1px; .a { margin: $x; }")
func TranspileLattice(source string) (string, error) {
	return TranspileLatticeFull(source, false, "  ")
}

// TranspileLatticeMinified compiles Lattice source to compact CSS with no extra whitespace.
//
// Example:
//
//	css, err := TranspileLatticeMinified("$x: 1px; .a { margin: $x; }")
//	// css = ".a{margin:1px;}"
func TranspileLatticeMinified(source string) (string, error) {
	return TranspileLatticeFull(source, true, "")
}

// ============================================================================
// Step-by-Step API
// ============================================================================

// TransformLatticeAST runs only the transformation step (Lattice AST → CSS AST).
//
// Use this if you already have a parsed ASTNode from the lattice-parser package,
// or if you want to inspect the CSS AST before serializing it.
//
// The input ast is modified in-place. Do not reuse it after this call.
//
// Example:
//
//	ast, _ := latticeparser.ParseLattice(source)
//	cssAST, err := TransformLatticeAST(ast)
//	// cssAST is now pure CSS (no Lattice constructs)
func TransformLatticeAST(ast *parser.ASTNode) (*parser.ASTNode, error) {
	transformer := NewLatticeTransformer()
	return transformer.Transform(ast)
}

// EmitCSS serializes a CSS AST to a pretty-printed string.
//
// Use this after TransformLatticeAST to produce the final CSS text.
//
// Example:
//
//	cssAST, _ := TransformLatticeAST(ast)
//	css := EmitCSS(cssAST)
func EmitCSS(ast *parser.ASTNode) string {
	emitter := NewCSSEmitter(false, "  ")
	return emitter.Emit(ast)
}

// EmitCSSMinified serializes a CSS AST to a compact string with no whitespace.
//
// Example:
//
//	cssAST, _ := TransformLatticeAST(ast)
//	css := EmitCSSMinified(cssAST)
//	// css = ".btn{color:red;}"
func EmitCSSMinified(ast *parser.ASTNode) string {
	emitter := NewCSSEmitter(true, "")
	return emitter.Emit(ast)
}
