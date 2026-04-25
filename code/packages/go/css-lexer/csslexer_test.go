package csslexer

import (
	"errors"
	"fmt"
	"testing"
)

func nonEOF(t *testing.T, source string) []Token {
	t.Helper()
	tokens, err := Tokenize(source)
	if err != nil {
		t.Fatalf("Tokenize failed: %v", err)
	}
	out := []Token{}
	for _, token := range tokens {
		if token.Type != "EOF" {
			out = append(out, token)
		}
	}
	return out
}

func tokenTypes(t *testing.T, source string) []string {
	tokens := nonEOF(t, source)
	out := make([]string, 0, len(tokens))
	for _, token := range tokens {
		out = append(out, token.Type)
	}
	return out
}

func tokenPairs(t *testing.T, source string) string {
	tokens := nonEOF(t, source)
	pairs := make([]string, 0, len(tokens))
	for _, token := range tokens {
		pairs = append(pairs, token.Type+"="+token.Value)
	}
	return fmt.Sprint(pairs)
}

func TestBasicTokens(t *testing.T) {
	got := tokenPairs(t, "color 42 3.14 .5 -42")
	want := "[IDENT=color NUMBER=42 NUMBER=3.14 NUMBER=.5 NUMBER=-42]"
	if got != want {
		t.Fatalf("basic token pairs = %s", got)
	}
	if got := tokenPairs(t, `"hello\nworld" 'world'`); got != "[STRING=hello\\nworld STRING=world]" {
		t.Fatalf("string token pairs = %s", got)
	}
	if got := tokenPairs(t, "#fff #header @media @-webkit-keyframes"); got != "[HASH=#fff HASH=#header AT_KEYWORD=@media AT_KEYWORD=@-webkit-keyframes]" {
		t.Fatalf("hash/at pairs = %s", got)
	}
}

func TestCompoundTokens(t *testing.T) {
	got := tokenPairs(t, "10px 2em 1.5rem -20px 50% -10% 10 px")
	want := "[DIMENSION=10px DIMENSION=2em DIMENSION=1.5rem DIMENSION=-20px PERCENTAGE=50% PERCENTAGE=-10% NUMBER=10 IDENT=px]"
	if got != want {
		t.Fatalf("compound pairs = %s", got)
	}
	if tokenTypes(t, "1e10")[0] != "DIMENSION" {
		t.Fatalf("expected 1e10 to begin with DIMENSION")
	}
}

func TestFunctionsUrlsCustomPropertiesAndUnicode(t *testing.T) {
	got := tokenPairs(t, "rgb( calc( linear-gradient( url(image.jpg) --main-color U+0025-00FF U+4??")
	want := "[FUNCTION=rgb( FUNCTION=calc( FUNCTION=linear-gradient( URL_TOKEN=url(image.jpg) CUSTOM_PROPERTY=--main-color UNICODE_RANGE=U+0025-00FF UNICODE_RANGE=U+4??]"
	if got != want {
		t.Fatalf("function pairs = %s", got)
	}
}

func TestOperatorsDelimitersAndComments(t *testing.T) {
	got := fmt.Sprint(tokenTypes(t, ":: ~= |= ^= $= *= { } ( ) [ ] ; : , . + > ~ * | ! / = & -"))
	want := "[COLON_COLON TILDE_EQUALS PIPE_EQUALS CARET_EQUALS DOLLAR_EQUALS STAR_EQUALS LBRACE RBRACE LPAREN RPAREN LBRACKET RBRACKET SEMICOLON COLON COMMA DOT PLUS GREATER TILDE STAR PIPE BANG SLASH EQUALS AMPERSAND MINUS]"
	if got != want {
		t.Fatalf("operators = %s", got)
	}

	got = fmt.Sprint(tokenTypes(t, "h1 /* selector */ {\n  color: red;\n}"))
	want = "[IDENT LBRACE IDENT COLON IDENT SEMICOLON RBRACE]"
	if got != want {
		t.Fatalf("comment skip = %s", got)
	}
}

func TestComplexCssAndPositions(t *testing.T) {
	tokens, err := Tokenize("h1 {\n  color: #333;\n  width: calc(100% - 20px);\n}")
	if err != nil {
		t.Fatalf("Tokenize failed: %v", err)
	}
	if tokens[0].Type != "IDENT" || tokens[0].Line != 1 || tokens[0].Column != 1 {
		t.Fatalf("first token position = %+v", tokens[0])
	}
	foundColor := false
	for _, token := range tokens {
		if token.Value == "color" {
			foundColor = token.Line == 2 && token.Column == 3
		}
	}
	if !foundColor {
		t.Fatalf("color position was not tracked")
	}
	got := fmt.Sprint(tokenTypes(t, "color: rgb(255, 0, 0);"))
	want := "[IDENT COLON FUNCTION NUMBER COMMA NUMBER COMMA NUMBER RPAREN SEMICOLON]"
	if got != want {
		t.Fatalf("rgb types = %s", got)
	}
}

func TestErrorRecoveryAndFactory(t *testing.T) {
	if got := fmt.Sprint(tokenTypes(t, `"unclosed string`)); got != "[BAD_STRING]" {
		t.Fatalf("bad string = %s", got)
	}
	if got := tokenPairs(t, "url(unclosed"); got != "[FUNCTION=url( IDENT=unclosed]" {
		t.Fatalf("bad url recovery = %s", got)
	}
	lexer := CreateLexer("a { }")
	tokens, err := lexer.Tokenize()
	if err != nil || tokens[len(tokens)-1].Type != "EOF" {
		t.Fatalf("factory lexer failed: %v", err)
	}
	_, err = Tokenize("`")
	var lexerErr *LexerError
	if !errors.As(err, &lexerErr) {
		t.Fatalf("expected LexerError, got %v", err)
	}
}
