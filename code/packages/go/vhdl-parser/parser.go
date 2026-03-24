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
// Locating the Grammar File
// -------------------------
//
// The vhdl.grammar file lives in code/grammars/, which is three directory
// levels above this source file (packages/go/vhdl-parser/ -> code/).
// The path is resolved at runtime using runtime.Caller(0) so it works
// regardless of the current working directory.
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
	"os"
	"path/filepath"
	"runtime"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	vhdllexer "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer"
)

// getGrammarPath resolves the absolute path to vhdl.grammar.
//
// Uses runtime.Caller(0) to find the directory containing this source file,
// then navigates three levels up to reach code/ and into grammars/.
//
// Directory layout:
//
//	code/
//	  grammars/
//	    vhdl.grammar       <-- target
//	  packages/
//	    go/
//	      vhdl-parser/
//	        parser.go      <-- we are here
//
// So from parser.go: ../../../grammars/vhdl.grammar
func getGrammarPath() string {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	return filepath.Join(root, "vhdl.grammar")
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
	// Step 1: Tokenize the VHDL source.
	// The lexer normalizes all identifiers and keywords to lowercase
	// (VHDL is case-insensitive) and produces a flat list of tokens
	// ending with EOF.
	tokens, err := vhdllexer.TokenizeVhdl(source)
	if err != nil {
		return nil, err
	}

	// Step 2: Read the grammar specification from disk.
	// The grammar file is a BNF-like text file that defines the syntax
	// rules for VHDL (entity_declaration, architecture_body, expression,
	// process_statement, etc.).
	bytes, err := os.ReadFile(getGrammarPath())
	if err != nil {
		return nil, err
	}

	// Step 3: Parse the grammar text into structured rule objects.
	// ParseParserGrammar returns a ParserGrammar containing a list of
	// GrammarRule objects, each with a name and body (the rule's definition
	// expressed as GrammarElement nodes: Sequence, Alternation, etc.).
	grammar, err := grammartools.ParseParserGrammar(string(bytes))
	if err != nil {
		return nil, err
	}

	// Step 4: Create the parser.
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
	vhdlParser, err := CreateVhdlParser(source)
	if err != nil {
		return nil, err
	}
	return vhdlParser.Parse()
}
