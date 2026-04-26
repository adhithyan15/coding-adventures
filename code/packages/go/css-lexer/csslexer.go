package csslexer

import (
	"fmt"
	"regexp"
	"strings"
	"unicode/utf8"
)

type Token struct {
	Type   string
	Value  string
	Line   int
	Column int
}

type LexerError struct {
	Line   int
	Column int
	Char   string
}

func (e *LexerError) Error() string {
	return fmt.Sprintf("unexpected character %q at %d:%d", e.Char, e.Line, e.Column)
}

type Lexer struct {
	source string
	offset int
	line   int
	column int
}

type pattern struct {
	tokenType string
	regex     *regexp.Regexp
	literal   string
	strip     bool
}

var skipPatterns = []*regexp.Regexp{
	regexp.MustCompile(`^/\*[\s\S]*?\*/`),
	regexp.MustCompile(`^[ \t\r\n]+`),
}

var tokenPatterns = []pattern{
	{tokenType: "STRING", regex: regexp.MustCompile(`^"([^"\\\n]|\\.)*"`), strip: true},
	{tokenType: "STRING", regex: regexp.MustCompile(`^'([^'\\\n]|\\.)*'`), strip: true},
	{tokenType: "DIMENSION", regex: regexp.MustCompile(`^-?[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+`)},
	{tokenType: "PERCENTAGE", regex: regexp.MustCompile(`^-?[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?%`)},
	{tokenType: "NUMBER", regex: regexp.MustCompile(`^-?[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?`)},
	{tokenType: "HASH", regex: regexp.MustCompile(`^#[a-zA-Z0-9_-]+`)},
	{tokenType: "AT_KEYWORD", regex: regexp.MustCompile(`^@-?[a-zA-Z][a-zA-Z0-9-]*`)},
	{tokenType: "URL_TOKEN", regex: regexp.MustCompile(`^url\([^)'"]*\)`)},
	{tokenType: "FUNCTION", regex: regexp.MustCompile(`^-?[a-zA-Z_][a-zA-Z0-9_-]*\(`)},
	{tokenType: "CDO", literal: "<!--"},
	{tokenType: "CDC", literal: "-->"},
	{tokenType: "UNICODE_RANGE", regex: regexp.MustCompile(`^[Uu]\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?`)},
	{tokenType: "CUSTOM_PROPERTY", regex: regexp.MustCompile(`^--[a-zA-Z_][a-zA-Z0-9_-]*`)},
	{tokenType: "IDENT", regex: regexp.MustCompile(`^-?[a-zA-Z_][a-zA-Z0-9_-]*`)},
	{tokenType: "COLON_COLON", literal: "::"},
	{tokenType: "TILDE_EQUALS", literal: "~="},
	{tokenType: "PIPE_EQUALS", literal: "|="},
	{tokenType: "CARET_EQUALS", literal: "^="},
	{tokenType: "DOLLAR_EQUALS", literal: "$="},
	{tokenType: "STAR_EQUALS", literal: "*="},
	{tokenType: "LBRACE", literal: "{"},
	{tokenType: "RBRACE", literal: "}"},
	{tokenType: "LPAREN", literal: "("},
	{tokenType: "RPAREN", literal: ")"},
	{tokenType: "LBRACKET", literal: "["},
	{tokenType: "RBRACKET", literal: "]"},
	{tokenType: "SEMICOLON", literal: ";"},
	{tokenType: "COLON", literal: ":"},
	{tokenType: "COMMA", literal: ","},
	{tokenType: "DOT", literal: "."},
	{tokenType: "PLUS", literal: "+"},
	{tokenType: "GREATER", literal: ">"},
	{tokenType: "TILDE", literal: "~"},
	{tokenType: "STAR", literal: "*"},
	{tokenType: "PIPE", literal: "|"},
	{tokenType: "BANG", literal: "!"},
	{tokenType: "SLASH", literal: "/"},
	{tokenType: "EQUALS", literal: "="},
	{tokenType: "AMPERSAND", literal: "&"},
	{tokenType: "MINUS", literal: "-"},
}

var errorPatterns = []pattern{
	{tokenType: "BAD_STRING", regex: regexp.MustCompile(`^"[^"]*$`)},
	{tokenType: "BAD_URL", regex: regexp.MustCompile(`^url\([^)]*$`)},
}

func New(source string) *Lexer {
	return &Lexer{source: source, line: 1, column: 1}
}

func CreateLexer(source string) *Lexer {
	return New(source)
}

func Tokenize(source string) ([]Token, error) {
	return New(source).Tokenize()
}

func (l *Lexer) Tokenize() ([]Token, error) {
	tokens := []Token{}
	for l.offset < len(l.source) {
		if l.skipIgnored() {
			continue
		}

		token, ok := l.matchPatterns(tokenPatterns)
		if !ok {
			token, ok = l.matchPatterns(errorPatterns)
		}
		if !ok {
			ch, _ := utf8.DecodeRuneInString(l.source[l.offset:])
			return nil, &LexerError{Line: l.line, Column: l.column, Char: string(ch)}
		}
		tokens = append(tokens, token)
	}

	tokens = append(tokens, Token{Type: "EOF", Value: "", Line: l.line, Column: l.column})
	return tokens, nil
}

func (l *Lexer) skipIgnored() bool {
	remaining := l.source[l.offset:]
	for _, regex := range skipPatterns {
		raw := regex.FindString(remaining)
		if raw != "" {
			l.advance(raw)
			return true
		}
	}
	return false
}

func (l *Lexer) matchPatterns(patterns []pattern) (Token, bool) {
	remaining := l.source[l.offset:]
	for _, candidate := range patterns {
		raw := ""
		if candidate.literal != "" {
			if strings.HasPrefix(remaining, candidate.literal) {
				raw = candidate.literal
			}
		} else {
			raw = candidate.regex.FindString(remaining)
		}
		if raw == "" {
			continue
		}

		value := raw
		if candidate.strip {
			value = raw[1 : len(raw)-1]
		}
		token := Token{Type: candidate.tokenType, Value: value, Line: l.line, Column: l.column}
		l.advance(raw)
		return token, true
	}
	return Token{}, false
}

func (l *Lexer) advance(text string) {
	for _, ch := range text {
		l.offset += len(string(ch))
		if ch == '\n' {
			l.line++
			l.column = 1
		} else {
			l.column++
		}
	}
}
