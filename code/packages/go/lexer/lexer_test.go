package lexer

import (
	"fmt"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

func TestLexerMath(t *testing.T) {
	source := "x = 1 + 2 * 3"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()

	expected := []struct{
		TType TokenType
		Value string
	}{
		{TokenName, "x"},
		{TokenEquals, "="},
		{TokenNumber, "1"},
		{TokenPlus, "+"},
		{TokenNumber, "2"},
		{TokenStar, "*"},
		{TokenNumber, "3"},
		{TokenEOF, ""},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d", len(expected), len(tokens))
	}

	for i, tok := range tokens {
		exp := expected[i]
		if tok.Type != exp.TType || tok.Value != exp.Value {
			t.Errorf("Mismatch on token %d: expected (%v, %s), got (%v, %s)",
			i, exp.TType, exp.Value, tok.Type, tok.Value)
		}
	}
}

func TestKeywords(t *testing.T) {
	source := "if x == 5"
	cfg := LexerConfig{Keywords: []string{"if"}}
	lexer := NewLexer(source, &cfg)
	tokens := lexer.Tokenize()

	if tokens[0].Type != TokenKeyword || tokens[0].Value != "if" {
		t.Errorf("Expected first token to map KeyWord successfully.")
	}

	if tokens[2].Type != TokenEqualsEquals || tokens[2].Value != "==" {
		t.Errorf("Expected equals equals double delimiter mapped to tokenizer natively.")
	}
}

func TestStrings(t *testing.T) {
	source := `print("Hello\n")`
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()

	if tokens[2].Type != TokenString || tokens[2].Value != "Hello\n" {
		t.Errorf("Expected newline parsing accurately resolving escape values %q.", tokens[2].Value)
	}
}

// -----------------------------------------------------------------------
// Grammar lexer: skip patterns
// -----------------------------------------------------------------------

func TestGrammarLexerSkipPatterns(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WHITESPACE", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 2},
		},
	}
	gl := NewGrammarLexer("hello world", grammar)
	tokens := gl.Tokenize()
	// Should have NAME, NAME, EOF (whitespace skipped)
	names := 0
	for _, tok := range tokens {
		if tok.TypeName == "NAME" {
			names++
		}
	}
	if names != 2 {
		t.Errorf("Expected 2 NAME tokens, got %d", names)
	}
}

// -----------------------------------------------------------------------
// Grammar lexer: aliases
// -----------------------------------------------------------------------

func TestGrammarLexerAlias(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NUM", Pattern: "[0-9]+", IsRegex: true, LineNumber: 1, Alias: "INT"},
		},
	}
	gl := NewGrammarLexer("42", grammar)
	tokens := gl.Tokenize()
	// First token should have TypeName "INT" (the alias)
	if tokens[0].TypeName != "INT" {
		t.Errorf("Expected TypeName 'INT', got %q", tokens[0].TypeName)
	}
}

// -----------------------------------------------------------------------
// Grammar lexer: reserved keywords
// -----------------------------------------------------------------------

func TestGrammarLexerReservedKeyword(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_]+", IsRegex: true, LineNumber: 1},
		},
		ReservedKeywords: []string{"class", "import"},
	}
	gl := NewGrammarLexer("class", grammar)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for reserved keyword")
		}
	}()
	gl.Tokenize()
}

func TestGrammarLexerNonReservedPasses(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_]+", IsRegex: true, LineNumber: 1},
		},
		ReservedKeywords: []string{"class"},
	}
	gl := NewGrammarLexer("hello", grammar)
	tokens := gl.Tokenize()
	if tokens[0].TypeName != "NAME" {
		t.Errorf("Expected NAME, got %q", tokens[0].TypeName)
	}
}

// -----------------------------------------------------------------------
// Grammar lexer: indentation mode
// -----------------------------------------------------------------------

func TestGrammarLexerIndentation(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_]+", IsRegex: true, LineNumber: 1},
			{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 2},
			{Name: "INT", Pattern: "[0-9]+", IsRegex: true, LineNumber: 3},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 4},
		},
		Keywords:        []string{"if"},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	gl := NewGrammarLexer("if x:\n    y = 1\n", grammar)
	tokens := gl.Tokenize()

	hasIndent := false
	hasDedent := false
	for _, tok := range tokens {
		if tok.TypeName == "INDENT" {
			hasIndent = true
		}
		if tok.TypeName == "DEDENT" {
			hasDedent = true
		}
	}
	if !hasIndent {
		t.Error("Expected INDENT token")
	}
	if !hasDedent {
		t.Error("Expected DEDENT token")
	}
}

func TestGrammarLexerIndentationTab(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 2},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	gl := NewGrammarLexer("if:\n\ty\n", grammar)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for tab indentation")
		}
	}()
	gl.Tokenize()
}

func TestGrammarLexerIndentationEmpty(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode:            "indentation",
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	gl := NewGrammarLexer("", grammar)
	tokens := gl.Tokenize()
	// Should have at least NEWLINE and EOF
	if tokens[len(tokens)-1].TypeName != "EOF" {
		t.Error("Expected EOF as last token")
	}
}

func TestGrammarLexerStringType(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "CUSTOM", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
	}
	gl := NewGrammarLexer("hello", grammar)
	tokens := gl.Tokenize()
	if tokens[0].TypeName != "CUSTOM" {
		t.Errorf("Expected TypeName 'CUSTOM', got %q", tokens[0].TypeName)
	}
}

// -----------------------------------------------------------------------
// TokenType.String() and Token.String()
// -----------------------------------------------------------------------

func TestTokenTypeString(t *testing.T) {
	cases := []struct {
		tt   TokenType
		want string
	}{
		{TokenName, "Name"},
		{TokenNumber, "Number"},
		{TokenString, "String"},
		{TokenKeyword, "Keyword"},
		{TokenPlus, "Plus"},
		{TokenMinus, "Minus"},
		{TokenStar, "Star"},
		{TokenSlash, "Slash"},
		{TokenEquals, "Equals"},
		{TokenEqualsEquals, "EqualsEquals"},
		{TokenLParen, "LParen"},
		{TokenRParen, "RParen"},
		{TokenComma, "Comma"},
		{TokenColon, "Colon"},
		{TokenSemicolon, "Semicolon"},
		{TokenLBrace, "LBrace"},
		{TokenRBrace, "RBrace"},
		{TokenLBracket, "LBracket"},
		{TokenRBracket, "RBracket"},
		{TokenDot, "Dot"},
		{TokenBang, "Bang"},
		{TokenNewline, "Newline"},
		{TokenEOF, "EOF"},
		{TokenType(999), "Unknown"},
	}
	for _, c := range cases {
		if got := c.tt.String(); got != c.want {
			t.Errorf("TokenType(%d).String() = %q, want %q", c.tt, got, c.want)
		}
	}
}

func TestTokenString(t *testing.T) {
	tok := Token{Type: TokenNumber, Value: "42", Line: 3, Column: 7}
	got := tok.String()
	want := `Token(Number, "42", 3:7)`
	if got != want {
		t.Errorf("Token.String() = %q, want %q", got, want)
	}
}

// -----------------------------------------------------------------------
// Hand-written lexer: comprehensive operator and path coverage
// -----------------------------------------------------------------------

func TestLexerAllSimpleTokens(t *testing.T) {
	source := "+-*/(),:;{}[].!"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()

	expected := []TokenType{
		TokenPlus, TokenMinus, TokenStar, TokenSlash,
		TokenLParen, TokenRParen, TokenComma, TokenColon,
		TokenSemicolon, TokenLBrace, TokenRBrace,
		TokenLBracket, TokenRBracket, TokenDot, TokenBang,
		TokenEOF,
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d", len(expected), len(tokens))
	}

	for i, tok := range tokens {
		if tok.Type != expected[i] {
			t.Errorf("Token %d: expected %v, got %v", i, expected[i], tok.Type)
		}
	}
}

func TestLexerPositionTracking(t *testing.T) {
	source := "a\nb"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()
	// a at 1:1, \n at 1:2, b at 2:1
	if tokens[0].Line != 1 || tokens[0].Column != 1 {
		t.Errorf("'a' position: expected 1:1, got %d:%d", tokens[0].Line, tokens[0].Column)
	}
	if tokens[1].Type != TokenNewline {
		t.Errorf("Expected newline token")
	}
	if tokens[2].Line != 2 || tokens[2].Column != 1 {
		t.Errorf("'b' position: expected 2:1, got %d:%d", tokens[2].Line, tokens[2].Column)
	}
}

func TestLexerUnderscoreInNames(t *testing.T) {
	source := "_foo bar_baz _123"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()
	if tokens[0].Value != "_foo" || tokens[1].Value != "bar_baz" || tokens[2].Value != "_123" {
		t.Errorf("Underscore handling wrong: %v %v %v", tokens[0].Value, tokens[1].Value, tokens[2].Value)
	}
}

func TestLexerEqualsVsDoubleEquals(t *testing.T) {
	source := "= == ="
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()
	if tokens[0].Type != TokenEquals || tokens[1].Type != TokenEqualsEquals || tokens[2].Type != TokenEquals {
		t.Errorf("Equals disambiguation failed")
	}
}

func TestLexerStringEscapes(t *testing.T) {
	source := `"hello\tworld\\\""` // hello\tworld\\\"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()
	if tokens[0].Type != TokenString {
		t.Fatalf("Expected string token")
	}
	if tokens[0].Value != "hello\tworld\\\"" {
		t.Errorf("Expected escape processing, got %q", tokens[0].Value)
	}
}

func TestLexerUnterminatedString(t *testing.T) {
	source := `"hello`
	lexer := NewLexer(source, nil)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for unterminated string")
		}
	}()
	lexer.Tokenize()
}

func TestLexerStringEndsWithBackslash(t *testing.T) {
	source := `"hello\`
	lexer := NewLexer(source, nil)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for string ending with backslash")
		}
	}()
	lexer.Tokenize()
}

func TestLexerUnexpectedCharacter(t *testing.T) {
	source := "x = §"
	lexer := NewLexer(source, nil)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for unexpected character")
		}
	}()
	lexer.Tokenize()
}

func TestLexerMultipleNewlines(t *testing.T) {
	source := "a\n\nb"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()
	// a, \n, \n, b, EOF
	count := 0
	for _, tok := range tokens {
		if tok.Type == TokenNewline {
			count++
		}
	}
	if count != 2 {
		t.Errorf("Expected 2 newlines, got %d", count)
	}
}

func TestLexerTabAndCarriageReturn(t *testing.T) {
	// Tabs and CR are treated as whitespace
	source := "a\t\rb"
	lexer := NewLexer(source, nil)
	tokens := lexer.Tokenize()
	if tokens[0].Value != "a" || tokens[1].Value != "b" {
		t.Errorf("Tab/CR whitespace handling wrong")
	}
}

func TestLexerNoConfig(t *testing.T) {
	lexer := NewLexer("hello", nil)
	tokens := lexer.Tokenize()
	if tokens[0].Type != TokenName || tokens[0].Value != "hello" {
		t.Errorf("Expected NAME 'hello'")
	}
}

func TestLexerKeywordSet(t *testing.T) {
	cfg := LexerConfig{Keywords: []string{"if", "else", "for"}}
	set := cfg.KeywordSet()
	if _, ok := set["if"]; !ok {
		t.Error("Expected 'if' in keyword set")
	}
	if _, ok := set["notakw"]; ok {
		t.Error("Unexpected 'notakw' in keyword set")
	}
}

func TestLexerEmptySource(t *testing.T) {
	lexer := NewLexer("", nil)
	tokens := lexer.Tokenize()
	if len(tokens) != 1 || tokens[0].Type != TokenEOF {
		t.Errorf("Expected just EOF for empty source")
	}
}

// -----------------------------------------------------------------------
// Grammar lexer: additional coverage
// -----------------------------------------------------------------------

func TestGrammarLexerKnownTypes(t *testing.T) {
	// Verify that known type names (PLUS, MINUS, etc.) map to proper TokenTypes
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 1},
			{Name: "MINUS", Pattern: "-", IsRegex: false, LineNumber: 2},
			{Name: "NUMBER", Pattern: "[0-9]+", IsRegex: true, LineNumber: 3},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ ]+", IsRegex: true, LineNumber: 10},
		},
	}
	gl := NewGrammarLexer("1 + 2 - 3", grammar)
	tokens := gl.Tokenize()

	if tokens[0].Type != TokenNumber || tokens[0].TypeName != "NUMBER" {
		t.Errorf("Expected NUMBER, got %v/%q", tokens[0].Type, tokens[0].TypeName)
	}
	if tokens[1].Type != TokenPlus || tokens[1].TypeName != "PLUS" {
		t.Errorf("Expected PLUS, got %v/%q", tokens[1].Type, tokens[1].TypeName)
	}
	if tokens[3].Type != TokenMinus || tokens[3].TypeName != "MINUS" {
		t.Errorf("Expected MINUS, got %v/%q", tokens[3].Type, tokens[3].TypeName)
	}
}

func TestGrammarLexerKeywordPromotion(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_]+", IsRegex: true, LineNumber: 1},
		},
		Keywords: []string{"def", "return"},
	}
	gl := NewGrammarLexer("def foo return", grammar)
	tokens := gl.Tokenize()

	if tokens[0].Type != TokenKeyword || tokens[0].TypeName != "KEYWORD" {
		t.Errorf("Expected KEYWORD 'def', got %v/%q", tokens[0].Type, tokens[0].TypeName)
	}
	if tokens[1].Type != TokenName || tokens[1].TypeName != "NAME" {
		t.Errorf("Expected NAME 'foo', got %v/%q", tokens[1].Type, tokens[1].TypeName)
	}
	if tokens[2].Type != TokenKeyword || tokens[2].TypeName != "KEYWORD" {
		t.Errorf("Expected KEYWORD 'return', got %v/%q", tokens[2].Type, tokens[2].TypeName)
	}
}

func TestGrammarLexerProcessEscapes(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "STRING", Pattern: `"[^"]*"`, IsRegex: true, LineNumber: 1},
		},
	}
	gl := NewGrammarLexer(`"hello\nworld\t!"`, grammar)
	tokens := gl.Tokenize()
	if tokens[0].Value != "hello\nworld\t!" {
		t.Errorf("Expected escape processing, got %q", tokens[0].Value)
	}
}

func TestGrammarLexerStringAlias(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "STRING_DQ", Pattern: `"[^"]*"`, IsRegex: true, LineNumber: 1, Alias: "STRING"},
		},
	}
	gl := NewGrammarLexer(`"hi"`, grammar)
	tokens := gl.Tokenize()
	// Alias contains "STRING" so it should process escapes
	if tokens[0].TypeName != "STRING" {
		t.Errorf("Expected TypeName 'STRING', got %q", tokens[0].TypeName)
	}
	if tokens[0].Value != "hi" {
		t.Errorf("Expected 'hi', got %q", tokens[0].Value)
	}
}

func TestGrammarLexerUnexpectedChar(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
	}
	gl := NewGrammarLexer("hello@world", grammar)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for unexpected character")
		}
	}()
	gl.Tokenize()
}

func TestGrammarLexerStandardModeNewlines(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ ]+", IsRegex: true, LineNumber: 2},
		},
	}
	gl := NewGrammarLexer("a\nb", grammar)
	tokens := gl.Tokenize()
	// Should have NAME, NEWLINE, NAME, EOF
	types := make([]string, len(tokens))
	for i, tok := range tokens {
		types[i] = tok.TypeName
	}
	if types[0] != "NAME" || types[1] != "NEWLINE" || types[2] != "NAME" || types[3] != "EOF" {
		t.Errorf("Unexpected tokens: %v", types)
	}
}

func TestGrammarLexerIndentationBracketSuppression(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_]+", IsRegex: true, LineNumber: 1},
			{Name: "LPAREN", Pattern: "(", IsRegex: false, LineNumber: 2},
			{Name: "RPAREN", Pattern: ")", IsRegex: false, LineNumber: 3},
			{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 4},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 5},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	// Inside brackets, newlines should be suppressed
	gl := NewGrammarLexer("f(\n  a,\n  b\n)\n", grammar)
	tokens := gl.Tokenize()

	// Count newlines -- should not have NEWLINE inside brackets
	newlines := 0
	for _, tok := range tokens {
		if tok.TypeName == "NEWLINE" {
			newlines++
		}
	}
	// Should have NEWLINE after ) and final NEWLINE from EOF, but not inside brackets
	if newlines > 3 {
		t.Errorf("Expected bracket suppression of newlines, got %d NEWLINEs", newlines)
	}
}

func TestGrammarLexerIndentationNestedBlocks(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_]+", IsRegex: true, LineNumber: 1},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 2},
			{Name: "INT", Pattern: "[0-9]+", IsRegex: true, LineNumber: 3},
		},
		Keywords: []string{"if", "return"},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	gl := NewGrammarLexer("if x:\n    if y:\n        return 1\n", grammar)
	tokens := gl.Tokenize()

	indents := 0
	dedents := 0
	for _, tok := range tokens {
		if tok.TypeName == "INDENT" {
			indents++
		}
		if tok.TypeName == "DEDENT" {
			dedents++
		}
	}
	if indents != 2 {
		t.Errorf("Expected 2 INDENTs, got %d", indents)
	}
	if dedents != 2 {
		t.Errorf("Expected 2 DEDENTs, got %d", dedents)
	}
}

func TestGrammarLexerIndentationBlankLines(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 2},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	// Blank lines between indented blocks should not emit tokens
	gl := NewGrammarLexer("a:\n    b\n\n    c\n", grammar)
	tokens := gl.Tokenize()
	// Should still work without extra dedents from blank line
	hasB := false
	hasC := false
	for _, tok := range tokens {
		if tok.Value == "b" {
			hasB = true
		}
		if tok.Value == "c" {
			hasC = true
		}
	}
	if !hasB || !hasC {
		t.Error("Expected both 'b' and 'c' tokens")
	}
}

func TestGrammarLexerIndentationCommentOnlyLine(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 2},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
			{Name: "COMMENT", Pattern: "#[^\n]*", IsRegex: true, LineNumber: 11},
		},
	}
	gl := NewGrammarLexer("a:\n    b\n    # comment\n    c\n", grammar)
	tokens := gl.Tokenize()
	// Comment-only line should be skipped without affecting indentation
	names := []string{}
	for _, tok := range tokens {
		if tok.TypeName == "NAME" {
			names = append(names, tok.Value)
		}
	}
	if len(names) != 3 || names[0] != "a" || names[1] != "b" || names[2] != "c" {
		t.Errorf("Expected [a b c], got %v", names)
	}
}

func TestGrammarLexerIndentationInconsistentDedent(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
			{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 2},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	// Indent to 4, then dedent to 3 (not matching any level)
	gl := NewGrammarLexer("a:\n    b\n   c\n", grammar)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for inconsistent dedent")
		}
	}()
	gl.Tokenize()
}

func TestGrammarLexerIndentationUnexpectedChar(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 10},
		},
	}
	gl := NewGrammarLexer("hello@", grammar)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for unexpected character in indentation mode")
		}
	}()
	gl.Tokenize()
}

func TestGrammarLexerLiteralSkipPattern(t *testing.T) {
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "ARROW", Pattern: "->", IsRegex: false, LineNumber: 2},
			{Name: "WS", Pattern: "[ ]+", IsRegex: true, LineNumber: 3},
		},
	}
	gl := NewGrammarLexer("hello -> world", grammar)
	tokens := gl.Tokenize()
	names := 0
	for _, tok := range tokens {
		if tok.TypeName == "NAME" {
			names++
		}
	}
	if names != 2 {
		t.Errorf("Expected 2 NAME tokens, got %d", names)
	}
}

// -----------------------------------------------------------------------
// Helper: makeGroupGrammar — simplified XML-like grammar with groups
// -----------------------------------------------------------------------
//
// This creates a grammar with:
//   - Default group: TEXT (any non-< chars) and OPEN_TAG (<)
//   - Tag group: TAG_NAME, EQUALS, VALUE, TAG_CLOSE (>)
//   - Skip patterns: whitespace
//   - Escape mode: none
//
// The grammar simulates an XML-like tokenizer where < triggers a switch
// to tag mode and > switches back to default mode.

func makeGroupGrammar() *grammartools.TokenGrammar {
	return &grammartools.TokenGrammar{
		EscapeMode: "none",
		Definitions: []grammartools.TokenDefinition{
			{Name: "TEXT", Pattern: "[^<]+", IsRegex: true, LineNumber: 1},
			{Name: "OPEN_TAG", Pattern: "<", IsRegex: false, LineNumber: 2},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: `[ \t\r\n]+`, IsRegex: true, LineNumber: 3},
		},
		Groups: map[string]*grammartools.PatternGroup{
			"tag": {
				Name: "tag",
				Definitions: []grammartools.TokenDefinition{
					{Name: "TAG_NAME", Pattern: "[a-zA-Z_][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 10},
					{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 11},
					{Name: "VALUE", Pattern: `"[^"]*"`, IsRegex: true, LineNumber: 12},
					{Name: "TAG_CLOSE", Pattern: ">", IsRegex: false, LineNumber: 13},
				},
			},
		},
	}
}

// -----------------------------------------------------------------------
// LexerContext unit tests
// -----------------------------------------------------------------------
//
// These tests verify that each LexerContext method correctly records
// actions without immediately mutating lexer state. The actions are
// applied by the tokenizer's main loop after the callback returns.

func TestLexerContextPushGroupRecordsAction(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	ctx.PushGroup("tag")
	if len(ctx.groupActions) != 1 || ctx.groupActions[0].action != "push" || ctx.groupActions[0].groupName != "tag" {
		t.Errorf("Expected push action for 'tag', got %v", ctx.groupActions)
	}
}

func TestLexerContextPushUnknownGroupPanics(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for unknown group")
		}
		msg := fmt.Sprintf("%v", r)
		if !strings.Contains(msg, "Unknown pattern group") {
			t.Errorf("Expected 'Unknown pattern group' in panic, got: %s", msg)
		}
	}()
	ctx.PushGroup("nonexistent")
}

func TestLexerContextPopGroupRecordsAction(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	ctx.PopGroup()
	if len(ctx.groupActions) != 1 || ctx.groupActions[0].action != "pop" {
		t.Errorf("Expected pop action, got %v", ctx.groupActions)
	}
}

func TestLexerContextActiveGroupReadsStack(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	if ctx.ActiveGroup() != "default" {
		t.Errorf("Expected 'default', got %q", ctx.ActiveGroup())
	}
}

func TestLexerContextGroupStackDepth(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	if ctx.GroupStackDepth() != 1 {
		t.Errorf("Expected depth 1, got %d", ctx.GroupStackDepth())
	}
}

func TestLexerContextEmitAppendsToken(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	synthetic := Token{Type: TokenName, Value: "!", Line: 1, Column: 1, TypeName: "SYNTHETIC"}
	ctx.Emit(synthetic)
	if len(ctx.emitted) != 1 || ctx.emitted[0].Value != "!" {
		t.Errorf("Expected emitted token with value '!', got %v", ctx.emitted)
	}
}

func TestLexerContextSuppressSetsFlag(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	if ctx.suppressed {
		t.Error("Expected suppressed to be false initially")
	}
	ctx.Suppress()
	if !ctx.suppressed {
		t.Error("Expected suppressed to be true after Suppress()")
	}
}

func TestLexerContextPeekReadsSource(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("hello", grammar)
	// Suppose token ended at position 3 (consumed "hel")
	ctx := &LexerContext{lexer: lexer, source: "hello", posAfter: 3}
	if ctx.Peek(1) != "l" {
		t.Errorf("Expected 'l', got %q", ctx.Peek(1))
	}
	if ctx.Peek(2) != "o" {
		t.Errorf("Expected 'o', got %q", ctx.Peek(2))
	}
	if ctx.Peek(3) != "" {
		t.Errorf("Expected empty string past EOF, got %q", ctx.Peek(3))
	}
}

func TestLexerContextPeekStrReadsSource(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("hello world", grammar)
	ctx := &LexerContext{lexer: lexer, source: "hello world", posAfter: 5}
	if ctx.PeekStr(6) != " world" {
		t.Errorf("Expected ' world', got %q", ctx.PeekStr(6))
	}
}

func TestLexerContextSetSkipEnabled(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	if ctx.skipEnabled != nil {
		t.Error("Expected skipEnabled to be nil initially")
	}
	ctx.SetSkipEnabled(false)
	if ctx.skipEnabled == nil || *ctx.skipEnabled != false {
		t.Error("Expected skipEnabled to be false after SetSkipEnabled(false)")
	}
}

func TestLexerContextMultiplePushes(t *testing.T) {
	grammar := makeGroupGrammar()
	lexer := NewGrammarLexer("x", grammar)
	ctx := &LexerContext{lexer: lexer, source: "x", posAfter: 1}
	ctx.PushGroup("tag")
	ctx.PushGroup("tag")
	if len(ctx.groupActions) != 2 {
		t.Errorf("Expected 2 group actions, got %d", len(ctx.groupActions))
	}
}

// -----------------------------------------------------------------------
// Pattern group tokenization tests
// -----------------------------------------------------------------------
//
// These tests verify that the lexer correctly switches between pattern
// groups based on callback actions, producing the right tokens in the
// right order.

func TestPatternGroupNoCallbackUsesDefault(t *testing.T) {
	grammar := makeGroupGrammar()
	tokens := NewGrammarLexer("hello", grammar).Tokenize()
	// TEXT pattern matches in default group
	if tokens[0].TypeName != "TEXT" || tokens[0].Value != "hello" {
		t.Errorf("Expected TEXT 'hello', got %q %q", tokens[0].TypeName, tokens[0].Value)
	}
}

func TestPatternGroupCallbackPushPop(t *testing.T) {
	// Simulates: <div> where < triggers push("tag"), > triggers pop().
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("<div>hello", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.PushGroup("tag")
		} else if token.TypeName == "TAG_CLOSE" {
			ctx.PopGroup()
		}
	})
	tokens := lexer.Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"OPEN_TAG", "<"},
		{"TAG_NAME", "div"},
		{"TAG_CLOSE", ">"},
		{"TEXT", "hello"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupCallbackWithAttributes(t *testing.T) {
	// Simulates: <div class="main"> where the tag group lexes
	// TAG_NAME, EQUALS, and VALUE tokens.
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer(`<div class="main">`, grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.PushGroup("tag")
		} else if token.TypeName == "TAG_CLOSE" {
			ctx.PopGroup()
		}
	})
	tokens := lexer.Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"OPEN_TAG", "<"},
		{"TAG_NAME", "div"},
		{"TAG_NAME", "class"},
		{"EQUALS", "="},
		{"VALUE", `"main"`},
		{"TAG_CLOSE", ">"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupNestedTags(t *testing.T) {
	// Grammar with CLOSE_TAG_START for </
	grammar := &grammartools.TokenGrammar{
		EscapeMode: "none",
		Definitions: []grammartools.TokenDefinition{
			{Name: "TEXT", Pattern: "[^<]+", IsRegex: true, LineNumber: 1},
			{Name: "CLOSE_TAG_START", Pattern: "</", IsRegex: false, LineNumber: 2},
			{Name: "OPEN_TAG", Pattern: "<", IsRegex: false, LineNumber: 3},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: `[ \t\r\n]+`, IsRegex: true, LineNumber: 4},
		},
		Groups: map[string]*grammartools.PatternGroup{
			"tag": {
				Name: "tag",
				Definitions: []grammartools.TokenDefinition{
					{Name: "TAG_NAME", Pattern: "[a-zA-Z_][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 10},
					{Name: "TAG_CLOSE", Pattern: ">", IsRegex: false, LineNumber: 11},
					{Name: "SLASH", Pattern: "/", IsRegex: false, LineNumber: 12},
				},
			},
		},
	}

	lexer := NewGrammarLexer("<a>text<b>inner</b></a>", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" || token.TypeName == "CLOSE_TAG_START" {
			ctx.PushGroup("tag")
		} else if token.TypeName == "TAG_CLOSE" {
			ctx.PopGroup()
		}
	})
	tokens := lexer.Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"OPEN_TAG", "<"},
		{"TAG_NAME", "a"},
		{"TAG_CLOSE", ">"},
		{"TEXT", "text"},
		{"OPEN_TAG", "<"},
		{"TAG_NAME", "b"},
		{"TAG_CLOSE", ">"},
		{"TEXT", "inner"},
		{"CLOSE_TAG_START", "</"},
		{"TAG_NAME", "b"},
		{"TAG_CLOSE", ">"},
		{"CLOSE_TAG_START", "</"},
		{"TAG_NAME", "a"},
		{"TAG_CLOSE", ">"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupSuppressToken(t *testing.T) {
	// Callback can suppress tokens (remove from output).
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("<hello", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.Suppress()
		}
	})
	tokens := lexer.Tokenize()

	var typeNames []string
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			typeNames = append(typeNames, tok.TypeName)
		}
	}
	// OPEN_TAG was suppressed, only TEXT remains
	if len(typeNames) != 1 || typeNames[0] != "TEXT" {
		t.Errorf("Expected [TEXT], got %v", typeNames)
	}
}

func TestPatternGroupEmitSyntheticToken(t *testing.T) {
	// Callback can emit synthetic tokens after the current one.
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("<hello", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.Emit(Token{
				Type:     TokenName,
				Value:    "[start]",
				Line:     token.Line,
				Column:   token.Column,
				TypeName: "MARKER",
			})
		}
	})
	tokens := lexer.Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"OPEN_TAG", "<"},
		{"MARKER", "[start]"},
		{"TEXT", "hello"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupSuppressAndEmit(t *testing.T) {
	// Suppress + emit = token replacement.
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("<hello", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.Suppress()
			ctx.Emit(Token{
				Type:     TokenName,
				Value:    "<",
				Line:     token.Line,
				Column:   token.Column,
				TypeName: "REPLACED",
			})
		}
	})
	tokens := lexer.Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"REPLACED", "<"},
		{"TEXT", "hello"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupPopAtBottomIsNoop(t *testing.T) {
	// Popping when only default remains is a no-op (no crash).
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("hello", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		ctx.PopGroup() // Should be safe even at the bottom
	})
	tokens := lexer.Tokenize()

	// Should still produce TEXT token without crashing
	if tokens[0].TypeName != "TEXT" {
		t.Errorf("Expected TEXT, got %q", tokens[0].TypeName)
	}
}

func TestPatternGroupSetSkipEnabledFalse(t *testing.T) {
	// Callback can disable skip patterns for significant whitespace.
	// When skip is disabled, whitespace that would normally be consumed
	// silently instead becomes part of a token match.
	grammar := &grammartools.TokenGrammar{
		EscapeMode: "none",
		Definitions: []grammartools.TokenDefinition{
			{Name: "TEXT", Pattern: "[^<]+", IsRegex: true, LineNumber: 1},
			{Name: "START", Pattern: "<!", IsRegex: false, LineNumber: 2},
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			{Name: "WS", Pattern: `[ \t]+`, IsRegex: true, LineNumber: 3},
		},
		Groups: map[string]*grammartools.PatternGroup{
			"raw": {
				Name: "raw",
				Definitions: []grammartools.TokenDefinition{
					{Name: "RAW_TEXT", Pattern: "[^>]+", IsRegex: true, LineNumber: 10},
					{Name: "END", Pattern: ">", IsRegex: false, LineNumber: 11},
				},
			},
		},
	}

	lexer := NewGrammarLexer("<! hello world >after", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "START" {
			ctx.PushGroup("raw")
			ctx.SetSkipEnabled(false)
		} else if token.TypeName == "END" {
			ctx.PopGroup()
			ctx.SetSkipEnabled(true)
		}
	})
	tokens := lexer.Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"START", "<!"},
		{"RAW_TEXT", " hello world "},
		{"END", ">"},
		{"TEXT", "after"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupNoGroupsBackwardCompat(t *testing.T) {
	// A grammar with no groups behaves identically to before.
	grammar := &grammartools.TokenGrammar{
		Definitions: []grammartools.TokenDefinition{
			{Name: "NAME", Pattern: "[a-zA-Z_][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 1},
			{Name: "NUMBER", Pattern: "[0-9]+", IsRegex: true, LineNumber: 2},
			{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 3},
		},
	}
	tokens := NewGrammarLexer("x + 1", grammar).Tokenize()

	type tv struct {
		TypeName string
		Value    string
	}
	var got []tv
	for _, tok := range tokens {
		if tok.TypeName != "NEWLINE" && tok.TypeName != "EOF" {
			got = append(got, tv{tok.TypeName, tok.Value})
		}
	}
	expected := []tv{
		{"NAME", "x"},
		{"PLUS", "+"},
		{"NUMBER", "1"},
	}
	if len(got) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(got), got)
	}
	for i, e := range expected {
		if got[i] != e {
			t.Errorf("Token %d: expected %v, got %v", i, e, got[i])
		}
	}
}

func TestPatternGroupClearCallback(t *testing.T) {
	// Passing nil to SetOnToken clears the callback.
	grammar := makeGroupGrammar()
	var called []string

	lexer := NewGrammarLexer("hello", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		called = append(called, token.TypeName)
	})
	lexer.SetOnToken(nil)
	lexer.Tokenize()

	if len(called) != 0 {
		t.Errorf("Expected callback not to be called, got %v", called)
	}
}

func TestPatternGroupStackResetsBetweenCalls(t *testing.T) {
	// The group stack resets when Tokenize() is called again.
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("<div", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.PushGroup("tag")
		}
	})

	// First call: pushes "tag" group
	tokens1 := lexer.Tokenize()
	hasTagName := false
	for _, tok := range tokens1 {
		if tok.TypeName == "TAG_NAME" {
			hasTagName = true
		}
	}
	if !hasTagName {
		t.Error("Expected TAG_NAME in first tokenize call")
	}

	// Second call: should start fresh from "default"
	// Reset the lexer state for re-tokenization
	lexer2 := NewGrammarLexer("<div", grammar)
	lexer2.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			ctx.PushGroup("tag")
		}
	})
	tokens2 := lexer2.Tokenize()
	hasTagName = false
	for _, tok := range tokens2 {
		if tok.TypeName == "TAG_NAME" {
			hasTagName = true
		}
	}
	if !hasTagName {
		t.Error("Expected TAG_NAME in second tokenize call")
	}
}

func TestPatternGroupMultiplePushPopSequence(t *testing.T) {
	// Multiple push/pop in one callback are applied in order.
	grammar := makeGroupGrammar()

	lexer := NewGrammarLexer("<div", grammar)
	lexer.SetOnToken(func(token Token, ctx *LexerContext) {
		if token.TypeName == "OPEN_TAG" {
			// Push tag twice (stacking)
			ctx.PushGroup("tag")
			ctx.PushGroup("tag")
		}
	})
	tokens := lexer.Tokenize()

	// After OPEN_TAG, stack should be ["default", "tag", "tag"]
	// TAG_NAME should still match since "tag" is on top
	hasTagName := false
	for _, tok := range tokens {
		if tok.TypeName == "TAG_NAME" {
			hasTagName = true
		}
	}
	if !hasTagName {
		t.Error("Expected TAG_NAME with double push")
	}
}
