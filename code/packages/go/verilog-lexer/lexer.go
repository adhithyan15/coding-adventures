// Package veriloglexer provides a grammar-driven tokenizer for Verilog HDL source code.
//
// Verilog is a Hardware Description Language (HDL) for describing digital
// circuits. This lexer tokenizes Verilog source using the verilog.tokens
// grammar file, which defines all token patterns (sized numbers, system
// identifiers, directives, operators, keywords, etc.).
//
// The Preprocessor
// ----------------
//
// Verilog has a C-like preprocessor with directives like `define, `ifdef,
// `include, and `timescale. These directives operate on raw text BEFORE
// tokenization. By default, CreateVerilogLexer and TokenizeVerilog apply
// the preprocessor. Use the Raw variants (CreateVerilogLexerRaw,
// TokenizeVerilogRaw) to skip preprocessing.
//
// Locating the Grammar File
// -------------------------
//
// The verilog.tokens file lives in code/grammars/, which is located
// relative to this source file at ../../grammars/verilog.tokens.
// The path is resolved at runtime using runtime.Caller.
package veriloglexer

import (
	"path/filepath"
	"runtime"

	cage "github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage"
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// getGrammarPath resolves the absolute path to verilog.tokens.
//
// Uses runtime.Caller to find the directory containing this source file,
// then navigates up to code/ and into grammars/. This approach works
// regardless of the working directory when tests or programs run.
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "verilog.tokens")
}

// CreateVerilogLexer creates a GrammarLexer configured for Verilog source code.
//
// The preprocessor runs first, expanding macros and evaluating conditionals.
// The preprocessed source is then passed to the grammar-driven lexer.
//
// Example:
//
//	lex, err := CreateVerilogLexer("module m; endmodule")
//	if err != nil { ... }
//	tokens := lex.Tokenize()
func CreateVerilogLexer(source string) (*lexer.GrammarLexer, error) {
	preprocessed := VerilogPreprocess(source)
	return createLexerFromSource(preprocessed)
}

// CreateVerilogLexerRaw creates a GrammarLexer without running the preprocessor.
//
// Use this when you want to tokenize raw Verilog source that has already
// been preprocessed, or when you want to see directives as tokens.
func CreateVerilogLexerRaw(source string) (*lexer.GrammarLexer, error) {
	return createLexerFromSource(source)
}

// createLexerFromSource loads the grammar and creates a GrammarLexer.
func createLexerFromSource(source string) (*lexer.GrammarLexer, error) {
	bytes, err := cage.ReadFileAt(Manifest, "code/grammars/verilog.tokens", getGrammarPath())
	if err != nil {
		return nil, err
	}
	grammar, err := grammartools.ParseTokenGrammar(string(bytes))
	if err != nil {
		return nil, err
	}
	return lexer.NewGrammarLexer(source, grammar), nil
}

// TokenizeVerilog tokenizes Verilog source code with preprocessing enabled.
//
// This is the main entry point. Pass in a string of Verilog source code,
// and get back a flat list of Token objects. The list always ends with
// an EOF token.
//
// Example:
//
//	tokens, err := TokenizeVerilog(`
//	    module and_gate(input a, input b, output y);
//	        assign y = a & b;
//	    endmodule
//	`)
//	// [Token(Keyword, "module", ...), Token(Name, "and_gate", ...), ...]
func TokenizeVerilog(source string) ([]lexer.Token, error) {
	verilogLexer, err := CreateVerilogLexer(source)
	if err != nil {
		return nil, err
	}
	return verilogLexer.Tokenize(), nil
}

// TokenizeVerilogRaw tokenizes Verilog source code without preprocessing.
//
// Directives like `define and `ifdef will appear as DIRECTIVE tokens
// in the output rather than being processed.
func TokenizeVerilogRaw(source string) ([]lexer.Token, error) {
	verilogLexer, err := CreateVerilogLexerRaw(source)
	if err != nil {
		return nil, err
	}
	return verilogLexer.Tokenize(), nil
}
