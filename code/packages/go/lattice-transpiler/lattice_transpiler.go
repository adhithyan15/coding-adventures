// Package latticetranspiler is the end-to-end Lattice-to-CSS pipeline.
//
// # What Is Lattice?
//
// Lattice is a CSS superset language (similar to Sass/SCSS) that adds
// compile-time variables, mixins, functions, control flow, and modules on
// top of standard CSS. Because Lattice compiles entirely at build time, the
// output is plain CSS that any browser understands — there is no runtime.
//
// # Package Role
//
// This package is the single entry point for consumers of the full pipeline.
// It wires together:
//
//  1. lattice-lexer  — tokenise source text
//  2. lattice-parser — parse token stream into a Lattice AST
//  3. lattice-ast-to-css — transform Lattice AST → CSS AST, then emit CSS text
//
// If you only need part of the pipeline (e.g., you already have a parsed AST),
// use the lower-level packages directly.
//
// # Quick Start
//
//	css, err := latticetranspiler.Transpile(`
//	    $primary: #4a90d9;
//	    .btn { color: $primary; }
//	`)
//	// css = ".btn {\n  color: #4a90d9;\n}\n"
//
// # Options
//
// Use [TranspileWithOptions] for full control over output formatting:
//
//	css, err := latticetranspiler.TranspileWithOptions(source, Options{
//	    Minify: true,
//	})
//	// css = ".btn{color:#4a90d9;}"
//
// # Error Handling
//
// All errors from the pipeline satisfy the standard error interface.
// Structured Lattice errors (undefined variable, wrong arity, etc.) can be
// extracted with errors.As:
//
//	css, err := latticetranspiler.Transpile(source)
//	var uve *latticeasttocss.UndefinedVariableError
//	if errors.As(err, &uve) {
//	    fmt.Printf("Undefined variable %s at line %d\n", uve.Name, uve.Line)
//	}
//
// # Dependency Chain
//
//	lattice-transpiler
//	  └── lattice-ast-to-css  (transformer, emitter, evaluator, scope, errors)
//	  └── lattice-parser
//	        └── lattice-lexer
//	              └── lexer, grammar-tools
//	        └── parser
package latticetranspiler

import (
	latticeasttocss "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-ast-to-css"
)

// Options controls how the transpiler formats its CSS output.
//
// The zero value of Options produces pretty-printed CSS with 2-space
// indentation, which is the most readable format for development.
//
// Example (pretty-print, default):
//
//	Options{}  →  Options{Minify: false, Indent: ""}
//	// Indent "" is replaced with the default "  " (two spaces)
//
// Example (minified):
//
//	Options{Minify: true}
//	// Indent is ignored when Minify is true
//
// Example (tab indentation):
//
//	Options{Indent: "\t"}
type Options struct {
	// Minify produces compact CSS with no extra whitespace.
	// When true, Indent is ignored.
	Minify bool

	// Indent is the string used for each level of indentation in
	// pretty-printed output. Common values: "  " (2 spaces), "    " (4 spaces),
	// "\t" (tab). If empty and Minify is false, defaults to "  " (2 spaces).
	Indent string
}

// resolveIndent returns the effective indentation string.
// An empty Indent with Minify=false defaults to two spaces, matching the
// conventional CSS style most developers expect.
func resolveIndent(opts Options) string {
	if opts.Minify {
		return ""
	}
	if opts.Indent == "" {
		return "  "
	}
	return opts.Indent
}

// Transpile compiles a Lattice source string to pretty-printed CSS.
//
// This is the most common entry point. Output uses 2-space indentation
// and newlines between rules.
//
// Example:
//
//	css, err := Transpile(`
//	    $color: red;
//	    .btn { color: $color; }
//	`)
//	// css = ".btn {\n  color: red;\n}\n"
func Transpile(source string) (string, error) {
	type transpileResult struct {
		css string
		err error
	}
	r, _ := StartNew[transpileResult]("lattice-transpiler.Transpile", transpileResult{},
		func(op *Operation[transpileResult], rf *ResultFactory[transpileResult]) *OperationResult[transpileResult] {
			css, err := latticeasttocss.TranspileLatticeFull(source, false, "  ")
			return rf.Generate(true, false, transpileResult{css, err})
		}).GetResult()
	return r.css, r.err
}

// TranspileMinified compiles a Lattice source string to compact CSS.
//
// No unnecessary whitespace is included in the output. Useful for
// production deployments where file size matters.
//
// Example:
//
//	css, err := TranspileMinified("$x: 1px; .a { margin: $x; }")
//	// css = ".a{margin:1px;}"
func TranspileMinified(source string) (string, error) {
	type transpileResult struct {
		css string
		err error
	}
	r, _ := StartNew[transpileResult]("lattice-transpiler.TranspileMinified", transpileResult{},
		func(op *Operation[transpileResult], rf *ResultFactory[transpileResult]) *OperationResult[transpileResult] {
			css, err := latticeasttocss.TranspileLatticeFull(source, true, "")
			return rf.Generate(true, false, transpileResult{css, err})
		}).GetResult()
	return r.css, r.err
}

// TranspileWithOptions compiles a Lattice source string to CSS using the
// given formatting options.
//
// This gives full control over output style. See [Options] for details.
//
// Example (4-space indentation):
//
//	css, err := TranspileWithOptions(source, Options{Indent: "    "})
//
// Example (minified):
//
//	css, err := TranspileWithOptions(source, Options{Minify: true})
func TranspileWithOptions(source string, opts Options) (string, error) {
	type transpileResult struct {
		css string
		err error
	}
	r, _ := StartNew[transpileResult]("lattice-transpiler.TranspileWithOptions", transpileResult{},
		func(op *Operation[transpileResult], rf *ResultFactory[transpileResult]) *OperationResult[transpileResult] {
			op.AddProperty("minify", opts.Minify)
			indent := resolveIndent(opts)
			css, err := latticeasttocss.TranspileLatticeFull(source, opts.Minify, indent)
			return rf.Generate(true, false, transpileResult{css, err})
		}).GetResult()
	return r.css, r.err
}
