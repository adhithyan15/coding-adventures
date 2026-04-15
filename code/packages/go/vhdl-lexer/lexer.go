// Package vhdllexer provides a grammar-driven tokenizer for VHDL source code.
//
// VHDL (VHSIC Hardware Description Language) is a Hardware Description Language
// designed by the US Department of Defense. Where Verilog is terse and C-like,
// VHDL is verbose and Ada-like, with strong typing, explicit declarations, and
// case-insensitive identifiers.
//
// Case Insensitivity
// ------------------
//
// VHDL is case-insensitive: ENTITY, Entity, and entity are all the same
// keyword. After the grammar-driven lexer produces tokens, this package
// normalizes all NAME and KEYWORD token values to lowercase. This means
// consumers never need to worry about case — they always see "entity",
// never "ENTITY" or "Entity".
//
// No Preprocessor
// ---------------
//
// Unlike Verilog (which has `define, `ifdef, etc.), VHDL has no preprocessor.
// Configuration and conditional compilation are handled through generics,
// generate statements, and configurations — all part of the language itself.
// This makes the lexer simpler: there is no preprocessing step.
//
// Locating the Grammar File
// -------------------------
//
// The vhdl.tokens file lives in code/grammars/, which is located relative
// to this source file at ../../../grammars/vhdl.tokens. The path is
// resolved at runtime using runtime.Caller.
package vhdllexer

import (
	"fmt"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	vhdlv1987 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer/internal/grammars/v1987"
	vhdlv1993 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer/internal/grammars/v1993"
	vhdlv2002 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer/internal/grammars/v2002"
	vhdlv2008 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer/internal/grammars/v2008"
	vhdlv2019 "github.com/adhithyan15/coding-adventures/code/packages/go/vhdl-lexer/internal/grammars/v2019"
)

// ============================================================================
// Grammar Path Resolution
// ============================================================================
//
// The grammar file (vhdl.tokens) defines all token patterns: keywords,
// operators, literals, identifiers, etc. We locate it relative to this
// source file using runtime.Caller(0), which gives us the absolute path
// to lexer.go at compile time. From there, we navigate three levels up
// (vhdl-lexer → go → packages → code) and into grammars/.

const DefaultVersion = "2008"

var supportedVersions = map[string]struct{}{
	"1987": {},
	"1993": {},
	"2002": {},
	"2008": {},
	"2019": {},
}

func ResolveVersion(version string) (string, error) {
	if version == "" {
		return DefaultVersion, nil
	}
	if _, ok := supportedVersions[version]; !ok {
		return "", fmt.Errorf("unknown VHDL version %q: valid versions are 1987, 1993, 2002, 2008, 2019", version)
	}
	return version, nil
}

func tokenGrammarForVersion(version string) (*grammartools.TokenGrammar, error) {
	resolved, err := ResolveVersion(version)
	if err != nil {
		return nil, err
	}

	switch resolved {
	case "1987":
		return vhdlv1987.TokenGrammarData, nil
	case "1993":
		return vhdlv1993.TokenGrammarData, nil
	case "2002":
		return vhdlv2002.TokenGrammarData, nil
	case "2008":
		return vhdlv2008.TokenGrammarData, nil
	case "2019":
		return vhdlv2019.TokenGrammarData, nil
	default:
		return nil, fmt.Errorf("compiled VHDL token grammar missing version %q", resolved)
	}
}

// ============================================================================
// Case Normalization
// ============================================================================
//
// VHDL is case-insensitive. The grammar file lists keywords in lowercase,
// but source code may use any case. We normalize NAME and KEYWORD token
// values to lowercase so that:
//
//   1. Keyword matching works correctly (the grammar engine matches against
//      lowercased keyword lists)
//   2. Consumers always get consistent lowercase values
//
// This normalization happens AFTER tokenization, as a post-processing step.
// Only NAME and KEYWORD tokens are affected — string literals, character
// literals, and other tokens preserve their original case.
//
// Truth table for normalization:
//
//   +------------+-------------+-------------+
//   | Token Type | Input       | Output      |
//   +------------+-------------+-------------+
//   | NAME       | "MySignal"  | "mysignal"  |
//   | KEYWORD    | "ENTITY"    | "entity"    |
//   | STRING     | "Hello"     | "Hello"     |
//   | NUMBER     | "42"        | "42"        |
//   +------------+-------------+-------------+

func normalizeCaseInsensitiveTokens(tokens []lexer.Token, keywordSet map[string]struct{}) []lexer.Token {
	for i := range tokens {
		switch tokens[i].TypeName {
		case "NAME":
			// Lowercase the identifier, then check if it's actually a keyword.
			// This handles VHDL's case insensitivity: "ENTITY" becomes "entity",
			// which matches the keyword list and gets reclassified as KEYWORD.
			tokens[i].Value = strings.ToLower(tokens[i].Value)
			if _, ok := keywordSet[tokens[i].Value]; ok {
				tokens[i].Type = lexer.TokenKeyword
				tokens[i].TypeName = "KEYWORD"
			}
		case "KEYWORD":
			// Already a keyword (matched lowercase in source), just normalize.
			tokens[i].Value = strings.ToLower(tokens[i].Value)
		}
		// All other token types (CHAR_LITERAL, BIT_STRING, BASED_LITERAL,
		// REAL_NUMBER, NUMBER, STRING, EXTENDED_IDENT, operators, delimiters)
		// preserve their original case.
	}
	return tokens
}

// ============================================================================
// Lexer Creation
// ============================================================================

// createLexerFromSource loads the VHDL grammar and creates a GrammarLexer.
//
// This is the internal workhorse: it reads the vhdl.tokens grammar file via
// op.File.ReadFile (enforcing the fs:read capability in required_capabilities.json),
// parses it into token patterns, and creates a GrammarLexer configured for
// VHDL source code.
func createLexerFromSource(source string) (*lexer.GrammarLexer, error) {
	return createLexerFromSourceVersion(source, DefaultVersion)
}

func createLexerFromSourceVersion(source string, version string) (*lexer.GrammarLexer, error) {
	grammar, err := tokenGrammarForVersion(version)
	if err != nil {
		return nil, err
	}

	return StartNew[*lexer.GrammarLexer]("vhdllexer.createLexerFromSource", nil,
		func(_ *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// CreateVhdlLexer creates a GrammarLexer configured for VHDL source code.
//
// The returned lexer is ready to tokenize. Call .Tokenize() on it to get
// the token list. Note that you will need to apply case normalization
// yourself if using this function directly — TokenizeVhdl does this
// automatically.
//
// Example:
//
//	lex, err := CreateVhdlLexer("entity e is end entity e;")
//	if err != nil { ... }
//	tokens := lex.Tokenize()
func CreateVhdlLexer(source string) (*lexer.GrammarLexer, error) {
	return CreateVhdlLexerVersion(source, DefaultVersion)
}

// CreateVhdlLexerVersion creates a GrammarLexer configured for the requested
// VHDL edition.
func CreateVhdlLexerVersion(source string, version string) (*lexer.GrammarLexer, error) {
	return createLexerFromSourceVersion(source, version)
}

// ============================================================================
// Tokenization
// ============================================================================

// TokenizeVhdl tokenizes VHDL source code with case normalization.
//
// This is the main entry point for tokenizing VHDL. It:
//  1. Creates a grammar-driven lexer from the vhdl.tokens grammar
//  2. Runs the lexer to produce raw tokens
//  3. Normalizes NAME and KEYWORD values to lowercase
//
// The result is a flat list of Token objects, always ending with an EOF token.
//
// Example:
//
//	tokens, err := TokenizeVhdl(`
//	    entity and_gate is
//	        port (
//	            a, b : in  std_logic;
//	            y    : out std_logic
//	        );
//	    end entity and_gate;
//	`)
//	// [Token(KEYWORD, "entity"), Token(NAME, "and_gate"), Token(KEYWORD, "is"), ...]
func TokenizeVhdl(source string) ([]lexer.Token, error) {
	return TokenizeVhdlVersion(source, DefaultVersion)
}

// TokenizeVhdlVersion tokenizes VHDL source code using the requested edition.
func TokenizeVhdlVersion(source string, version string) ([]lexer.Token, error) {
	grammar, err := tokenGrammarForVersion(version)
	if err != nil {
		return nil, err
	}

	return StartNew[[]lexer.Token]("vhdllexer.TokenizeVhdl", nil,
		func(_ *Operation[[]lexer.Token], rf *ResultFactory[[]lexer.Token]) *OperationResult[[]lexer.Token] {
			// Build the keyword set for post-tokenization reclassification.
			// The grammar lexer only matches keywords by exact string equality,
			// so "ENTITY" won't match "entity" in the keyword list. Our
			// normalization step lowercases NAME tokens and checks this set
			// to promote them to KEYWORD when they match.
			keywordSet := make(map[string]struct{})
			for _, kw := range grammar.Keywords {
				keywordSet[kw] = struct{}{}
			}

			vhdlLexer := lexer.NewGrammarLexer(source, grammar)
			tokens := vhdlLexer.Tokenize()
			tokens = normalizeCaseInsensitiveTokens(tokens, keywordSet)
			return rf.Generate(true, false, tokens)
		}).GetResult()
}
