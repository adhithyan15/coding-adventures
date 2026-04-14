// Package veriloglexer provides a grammar-driven tokenizer for Verilog HDL source code.
//
// Verilog is a Hardware Description Language (HDL) for describing digital
// circuits. This lexer tokenizes Verilog source using compiled token grammars
// for the supported IEEE editions rather than reading grammar files from disk
// at runtime. That keeps the package friendly to restricted targets and makes
// edition selection explicit.
//
// The Preprocessor
// ----------------
//
// Verilog has a C-like preprocessor with directives like `define, `ifdef,
// `include, and `timescale. These directives operate on raw text BEFORE
// tokenization. By default, CreateVerilogLexer and TokenizeVerilog apply
// the preprocessor. Use the Raw variants (CreateVerilogLexerRaw,
// TokenizeVerilogRaw) to skip preprocessing.
package veriloglexer

import (
	"fmt"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	verilogv1995 "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-lexer/internal/grammars/v1995"
	verilogv2001 "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-lexer/internal/grammars/v2001"
	verilogv2005 "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-lexer/internal/grammars/v2005"
)

const DefaultVersion = "2005"

var supportedVersions = map[string]struct{}{
	"1995": {},
	"2001": {},
	"2005": {},
}

func ResolveVersion(version string) (string, error) {
	if version == "" {
		return DefaultVersion, nil
	}
	if _, ok := supportedVersions[version]; !ok {
		return "", fmt.Errorf("unknown Verilog version %q: valid versions are 1995, 2001, 2005", version)
	}
	return version, nil
}

func tokenGrammarForVersion(version string) (*grammartools.TokenGrammar, error) {
	resolved, err := ResolveVersion(version)
	if err != nil {
		return nil, err
	}

	switch resolved {
	case "1995":
		return verilogv1995.TokenGrammarData, nil
	case "2001":
		return verilogv2001.TokenGrammarData, nil
	case "2005":
		return verilogv2005.TokenGrammarData, nil
	default:
		return nil, fmt.Errorf("compiled Verilog token grammar missing version %q", resolved)
	}
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
	return CreateVerilogLexerVersion(source, DefaultVersion)
}

// CreateVerilogLexerRaw creates a GrammarLexer without running the preprocessor.
//
// Use this when you want to tokenize raw Verilog source that has already
// been preprocessed, or when you want to see directives as tokens.
func CreateVerilogLexerRaw(source string) (*lexer.GrammarLexer, error) {
	return CreateVerilogLexerRawVersion(source, DefaultVersion)
}

// CreateVerilogLexerVersion creates a GrammarLexer for the requested
// Verilog edition, running the preprocessor first.
func CreateVerilogLexerVersion(source string, version string) (*lexer.GrammarLexer, error) {
	preprocessed := VerilogPreprocess(source)
	return createLexerFromSource(preprocessed, version)
}

// CreateVerilogLexerRawVersion creates a GrammarLexer for the requested
// Verilog edition without preprocessing.
func CreateVerilogLexerRawVersion(source string, version string) (*lexer.GrammarLexer, error) {
	return createLexerFromSource(source, version)
}

// createLexerFromSource selects the compiled token grammar for the requested
// edition and creates a GrammarLexer from it.
func createLexerFromSource(source string, version string) (*lexer.GrammarLexer, error) {
	grammar, err := tokenGrammarForVersion(version)
	if err != nil {
		return nil, err
	}

	return StartNew[*lexer.GrammarLexer]("veriloglexer.createLexerFromSource", nil,
		func(_ *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
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
	return TokenizeVerilogVersion(source, DefaultVersion)
}

// TokenizeVerilogVersion tokenizes Verilog source code using the requested
// edition, with preprocessing enabled.
func TokenizeVerilogVersion(source string, version string) ([]lexer.Token, error) {
	verilogLexer, err := CreateVerilogLexerVersion(source, version)
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
	return TokenizeVerilogRawVersion(source, DefaultVersion)
}

// TokenizeVerilogRawVersion tokenizes Verilog source code using the requested
// edition without preprocessing.
func TokenizeVerilogRawVersion(source string, version string) ([]lexer.Token, error) {
	verilogLexer, err := CreateVerilogLexerRawVersion(source, version)
	if err != nil {
		return nil, err
	}
	return verilogLexer.Tokenize(), nil
}
