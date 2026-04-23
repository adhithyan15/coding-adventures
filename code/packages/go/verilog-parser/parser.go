// Package verilogparser provides a grammar-driven parser for Verilog HDL source code.
//
// Verilog is a Hardware Description Language (HDL) for describing digital
// circuits. This parser takes Verilog source code and produces an Abstract
// Syntax Tree (AST) that represents the hierarchical structure of the design:
// modules, ports, assignments, always blocks, expressions, and so on.
//
// Architecture
// ------------
//
// The parser is a thin wrapper around two lower-level packages:
//
//  1. verilog-lexer — tokenizes the raw source text into a flat list of
//     tokens (keywords, names, operators, numbers, etc.). The lexer also
//     runs the Verilog preprocessor, expanding `define macros and evaluating
//     `ifdef conditionals before tokenization.
//
//  2. parser.GrammarParser — a generic packrat parser that interprets a
//     grammar specification at runtime. Given a list of tokens and a set
//     of grammar rules, it produces an ASTNode tree. The grammar rules
//     live in verilog.grammar, a human-readable BNF-like file.
//
// This package glues them together: tokenize with verilog-lexer, load
// verilog.grammar, hand both to GrammarParser, and return the result.
//
// Locating the Grammar File
// -------------------------
//
// The verilog.grammar file lives in code/grammars/, which is three directory
// levels above this source file (packages/go/verilog-parser/ -> code/).
// The path is resolved at runtime using runtime.Caller(0) so it works
// regardless of the current working directory.
//
// Usage
// -----
//
//	// Parse a simple module:
//	ast, err := verilogparser.ParseVerilog(`
//	    module and_gate(input a, input b, output y);
//	        assign y = a & b;
//	    endmodule
//	`)
//	if err != nil { log.Fatal(err) }
//	// ast.RuleName == "source_text"
//	// ast.Children contains the parsed module declaration
package verilogparser

import (
	"fmt"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	veriloglexer "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-lexer"
	verilogv1995 "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-parser/internal/grammars/v1995"
	verilogv2001 "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-parser/internal/grammars/v2001"
	verilogv2005 "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-parser/internal/grammars/v2005"
)

const DefaultVersion = veriloglexer.DefaultVersion

func parserGrammarForVersion(version string) (*grammartools.ParserGrammar, error) {
	resolved, err := veriloglexer.ResolveVersion(version)
	if err != nil {
		return nil, err
	}

	switch resolved {
	case "1995":
		return verilogv1995.ParserGrammarData, nil
	case "2001":
		return verilogv2001.ParserGrammarData, nil
	case "2005":
		return verilogv2005.ParserGrammarData, nil
	default:
		return nil, fmt.Errorf("compiled Verilog parser grammar missing version %q", resolved)
	}
}

// CreateVerilogParser tokenizes Verilog source and returns a configured
// GrammarParser ready to parse.
//
// This is the two-step API: you get back the parser object and can call
// .Parse() yourself. Useful when you want to inspect the parser state
// or grammar before parsing.
//
// Steps:
//  1. Tokenize the source using verilog-lexer (includes preprocessing).
//  2. Read the verilog.grammar file from disk.
//  3. Parse the grammar file into a ParserGrammar (the rule set).
//  4. Create a GrammarParser with the tokens and grammar.
//
// Example:
//
//	p, err := CreateVerilogParser("module m; endmodule")
//	if err != nil { ... }
//	ast, err := p.Parse()
func CreateVerilogParser(source string) (*parser.GrammarParser, error) {
	return CreateVerilogParserVersion(source, DefaultVersion)
}

// CreateVerilogParserVersion tokenizes Verilog source and returns a configured
// GrammarParser for the requested Verilog edition.
func CreateVerilogParserVersion(source string, version string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the Verilog source.
	// The lexer handles the preprocessor (`define, `ifdef, etc.) and
	// produces a flat list of tokens ending with EOF.
	tokens, err := veriloglexer.TokenizeVerilogVersion(source, version)
	if err != nil {
		return nil, err
	}

	grammar, err := parserGrammarForVersion(version)
	if err != nil {
		return nil, err
	}

	return StartNew[*parser.GrammarParser]("verilogparser.CreateVerilogParser", nil,
		func(_ *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			// Step 2: Create the parser from the compiled grammar.
			// NewGrammarParser builds a packrat parser with memoization that
			// will interpret the grammar rules against the token stream.
			return rf.Generate(true, false, parser.NewGrammarParser(tokens, grammar))
		}).GetResult()
}

// ParseVerilog tokenizes and parses Verilog source code in one step.
//
// This is the convenience API: pass in source, get back an AST.
// The returned ASTNode's RuleName will be "source_text" (the grammar's
// entry rule), and its Children will contain the parsed module declarations.
//
// Example:
//
//	ast, err := ParseVerilog(`
//	    module counter(input clk, input reset, output [7:0] count);
//	        reg [7:0] count;
//	        always @(posedge clk)
//	            if (reset) count <= 0;
//	            else count <= count + 1;
//	    endmodule
//	`)
//	if err != nil { log.Fatal(err) }
//	// Walk ast.Children to inspect the parsed structure
func ParseVerilog(source string) (*parser.ASTNode, error) {
	return ParseVerilogVersion(source, DefaultVersion)
}

// ParseVerilogVersion tokenizes and parses Verilog source code using the
// requested Verilog edition.
func ParseVerilogVersion(source string, version string) (*parser.ASTNode, error) {
	verilogParser, err := CreateVerilogParserVersion(source, version)
	if err != nil {
		return nil, err
	}
	return verilogParser.Parse()
}
