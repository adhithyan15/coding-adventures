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
//   1. verilog-lexer — tokenizes the raw source text into a flat list of
//      tokens (keywords, names, operators, numbers, etc.). The lexer also
//      runs the Verilog preprocessor, expanding `define macros and evaluating
//      `ifdef conditionals before tokenization.
//
//   2. parser.GrammarParser — a generic packrat parser that interprets a
//      grammar specification at runtime. Given a list of tokens and a set
//      of grammar rules, it produces an ASTNode tree. The grammar rules
//      live in verilog.grammar, a human-readable BNF-like file.
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
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	veriloglexer "github.com/adhithyan15/coding-adventures/code/packages/go/verilog-lexer"
)

// getGrammarPath resolves the absolute path to verilog.grammar.
//
// Uses runtime.Caller(0) to find the directory containing this source file,
// then navigates three levels up to reach code/ and into grammars/.
//
// Directory layout:
//
//	code/
//	  grammars/
//	    verilog.grammar    <-- target
//	  packages/
//	    go/
//	      verilog-parser/
//	        parser.go      <-- we are here
//
// So from parser.go: ../../../grammars/verilog.grammar
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "verilog.grammar")
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
	// Step 1: Tokenize the Verilog source.
	// The lexer handles the preprocessor (`define, `ifdef, etc.) and
	// produces a flat list of tokens ending with EOF.
	tokens, err := veriloglexer.TokenizeVerilog(source)
	if err != nil {
		return nil, err
	}

	return StartNew[*parser.GrammarParser]("verilogparser.CreateVerilogParser", nil,
		func(op *Operation[*parser.GrammarParser], rf *ResultFactory[*parser.GrammarParser]) *OperationResult[*parser.GrammarParser] {
			// Step 2: Read the grammar specification from disk.
			// The grammar file is a BNF-like text file that defines the syntax
			// rules for Verilog (module_declaration, expression, statement, etc.).
			bytes, err := op.File.ReadFile(getGrammarPath())
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 3: Parse the grammar text into structured rule objects.
			// ParseParserGrammar returns a ParserGrammar containing a list of
			// GrammarRule objects, each with a name and body (the rule's definition
			// expressed as GrammarElement nodes: Sequence, Alternation, etc.).
			grammar, err := grammartools.ParseParserGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}

			// Step 4: Create the parser.
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
	verilogParser, err := CreateVerilogParser(source)
	if err != nil {
		return nil, err
	}
	return verilogParser.Parse()
}
