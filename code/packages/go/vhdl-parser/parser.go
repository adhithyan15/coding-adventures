// Package vhdlparser provides a grammar-driven parser for VHDL source code.
//
// VHDL (VHSIC Hardware Description Language) is a Hardware Description Language
// used to model digital circuits. Unlike Verilog (which is terse, like C), VHDL
// is verbose and strongly typed (like Ada). Every signal must be declared with a
// type. Every entity has a separate architecture. Every port specifies direction
// and type.
//
// This parser takes VHDL source code and produces an Abstract Syntax Tree (AST)
// that represents the hierarchical structure of the design: entities, architectures,
// ports, signal assignments, processes, if/elsif/else chains, and so on.
//
// Architecture
// ------------
//
// The parser is a thin wrapper around two lower-level packages:
//
//  1. vhdl-lexer — tokenizes the raw source text into a flat list of tokens
//     (keywords, names, operators, numbers, etc.). VHDL is case-insensitive,
//     and the lexer normalizes all identifiers and keywords to lowercase.
//
//  2. parser.GrammarParser — a generic packrat parser that interprets a grammar
//     specification at runtime. Given a list of tokens and a set of grammar
//     rules, it produces an ASTNode tree. The grammar rules live in
//     vhdl.grammar, a human-readable BNF-like file.
//
// This package glues them together: tokenize with vhdl-lexer, load
// vhdl.grammar, hand both to GrammarParser, and return the result.
//
// VHDL vs Verilog (Key Structural Differences)
// ---------------------------------------------
//
// In Verilog, a single "module" declaration contains both the interface (ports)
// and the implementation (body). In VHDL, these are separated:
//
//   - entity  = the interface (ports, generics) — like a module header
//   - architecture = the implementation — like a module body
//
// A single entity can have multiple architectures (e.g., behavioral vs structural),
// which is powerful for simulation and verification.
//
// Grammar Embedding
// -----------------
//
// The VHDL parser grammars are embedded at compile time as native Go data in
// the internal/grammars/v* subpackages (ParserGrammarData). Nothing is read
// from code/grammars/ at run time, so the parser works when built standalone
// and needs no filesystem capability.
//
// Usage
// -----
//
//	// Parse a simple entity + architecture:
//	ast, err := vhdlparser.ParseVhdl(`
//	    entity and_gate is
//	        port (a, b : in std_logic; y : out std_logic);
//	    end entity and_gate;
//	    architecture rtl of and_gate is
//	    begin
//	        y <= a and b;
//	    end architecture rtl;
//	`)
//	if err != nil { log.Fatal(err) }
//	// ast.RuleName == "design_file"
//	// ast.Children contains the parsed design units
package vhdlparser

import (
	"fmt"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	vhdllexer "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer"
	vhdlv1987 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-parser/internal/grammars/v1987"
	vhdlv1993 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-parser/internal/grammars/v1993"
	vhdlv2002 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-parser/internal/grammars/v2002"
	vhdlv2008 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-parser/internal/grammars/v2008"
	vhdlv2019 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-parser/internal/grammars/v2019"
)

// DefaultVersion is the VHDL edition used when no version is specified.
const DefaultVersion = vhdllexer.DefaultVersion

func parserGrammarForVersion(version string) (*grammartools.ParserGrammar, error) {
	resolved, err := vhdllexer.ResolveVersion(version)
	if err != nil {
		return nil, err
	}

	switch resolved {
	case "1987":
		return vhdlv1987.ParserGrammarData, nil
	case "1993":
		return vhdlv1993.ParserGrammarData, nil
	case "2002":
		return vhdlv2002.ParserGrammarData, nil
	case "2008":
		return vhdlv2008.ParserGrammarData, nil
	case "2019":
		return vhdlv2019.ParserGrammarData, nil
	default:
		return nil, fmt.Errorf("compiled VHDL parser grammar missing version %q", resolved)
	}
}

// CreateVhdlParser tokenizes VHDL source and returns a configured
// GrammarParser ready to parse.
//
// This is the two-step API: you get back the parser object and can call
// .Parse() yourself. Useful when you want to inspect the parser state
// or grammar before parsing.
//
// Steps:
//  1. Tokenize the source using vhdl-lexer (handles case normalization).
//  2. Read the vhdl.grammar file from disk.
//  3. Parse the grammar file into a ParserGrammar (the rule set).
//  4. Create a GrammarParser with the tokens and grammar.
//
// Example:
//
//	p, err := CreateVhdlParser("entity e is end entity e;")
//	if err != nil { ... }
//	ast, err := p.Parse()
func CreateVhdlParser(source string) (*parser.GrammarParser, error) {
	return CreateVhdlParserVersion(source, DefaultVersion)
}

// CreateVhdlParserVersion tokenizes VHDL source and returns a configured
// GrammarParser for the requested VHDL edition.
func CreateVhdlParserVersion(source string, version string) (*parser.GrammarParser, error) {
	// Step 1: Tokenize the VHDL source.
	// The lexer normalizes all identifiers and keywords to lowercase
	// (VHDL is case-insensitive) and produces a flat list of tokens
	// ending with EOF.
	tokens, err := vhdllexer.TokenizeVhdlVersion(source, version)
	if err != nil {
		return nil, err
	}

	grammar, err := parserGrammarForVersion(version)
	if err != nil {
		return nil, err
	}

	// Step 2: Create the parser from the compiled grammar.
	// NewGrammarParser builds a packrat parser with memoization that
	// will interpret the grammar rules against the token stream.
	return parser.NewGrammarParser(tokens, grammar), nil
}

// ParseVhdl tokenizes and parses VHDL source code in one step.
//
// This is the convenience API: pass in source, get back an AST.
// The returned ASTNode's RuleName will be "design_file" (the grammar's
// entry rule), and its Children will contain the parsed design units
// (entities, architectures, packages, etc.).
//
// Example:
//
//	ast, err := ParseVhdl(`
//	    entity counter is
//	        port (clk, reset : in std_logic;
//	              count : out std_logic_vector(7 downto 0));
//	    end entity counter;
//
//	    architecture behavioral of counter is
//	        signal count_reg : std_logic_vector(7 downto 0);
//	    begin
//	        process (clk)
//	        begin
//	            if rising_edge(clk) then
//	                if reset = '1' then
//	                    count_reg <= "00000000";
//	                else
//	                    count_reg <= count_reg + 1;
//	                end if;
//	            end if;
//	        end process;
//	        count <= count_reg;
//	    end architecture behavioral;
//	`)
//	if err != nil { log.Fatal(err) }
//	// Walk ast.Children to inspect the parsed structure
func ParseVhdl(source string) (*parser.ASTNode, error) {
	return ParseVhdlVersion(source, DefaultVersion)
}

// ParseVhdlVersion tokenizes and parses VHDL source code using the requested
// VHDL edition.
func ParseVhdlVersion(source string, version string) (*parser.ASTNode, error) {
	vhdlParser, err := CreateVhdlParserVersion(source, version)
	if err != nil {
		return nil, err
	}
	return vhdlParser.Parse()
}
