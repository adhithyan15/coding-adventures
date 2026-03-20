package lexer

import (
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
