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
