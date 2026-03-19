// Package lexer implements character-by-character analysis translating source bytes to strict Typed Tokens.
package lexer

import (
	"fmt"
	"strings"
	"unicode"
)

var SimpleTokens = map[rune]TokenType{
	'+': TokenPlus,
	'-': TokenMinus,
	'*': TokenStar,
	'/': TokenSlash,
	'(': TokenLParen,
	')': TokenRParen,
	',': TokenComma,
	':': TokenColon,
	';': TokenSemicolon,
	'{': TokenLBrace,
	'}': TokenRBrace,
	'[': TokenLBracket,
	']': TokenRBracket,
	'.': TokenDot,
	'!': TokenBang,
}

type LexerConfig struct {
	Keywords []string
}

func (c LexerConfig) KeywordSet() map[string]struct{} {
	set := make(map[string]struct{})
	for _, k := range c.Keywords {
		set[k] = struct{}{}
	}
	return set
}

type Lexer struct {
	source      string
	config      LexerConfig
	pos         int
	line        int
	column      int
	tokens      []Token
	keywordsSet map[string]struct{}
}

func NewLexer(source string, config *LexerConfig) *Lexer {
	cfg := LexerConfig{}
	if config != nil {
		cfg = *config
	}
	return &Lexer{
		source:      source,
		config:      cfg,
		pos:         0,
		line:        1,
		column:      1,
		tokens:      []Token{},
		keywordsSet: cfg.KeywordSet(),
	}
}

func (l *Lexer) currentChar() (rune, bool) {
	if l.pos < len(l.source) {
		return rune(l.source[l.pos]), true
	}
	return 0, false
}

func (l *Lexer) peek() (rune, bool) {
	if l.pos+1 < len(l.source) {
		return rune(l.source[l.pos+1]), true
	}
	return 0, false
}

func (l *Lexer) advance() rune {
	char := rune(l.source[l.pos])
	l.pos++
	if char == '\n' {
		l.line++
		l.column = 1
	} else {
		l.column++
	}
	return char
}

func (l *Lexer) skipWhitespace() {
	for {
		c, ok := l.currentChar()
		if !ok || (c != ' ' && c != '\t' && c != '\r') {
			break
		}
		l.advance()
	}
}

func (l *Lexer) readNumber() Token {
	startLine := l.line
	startCol := l.column
	var sb strings.Builder
	for {
		c, ok := l.currentChar()
		if !ok || !unicode.IsDigit(c) {
			break
		}
		sb.WriteRune(l.advance())
	}
	return Token{Type: TokenNumber, Value: sb.String(), Line: startLine, Column: startCol}
}

func (l *Lexer) readName() Token {
	startLine := l.line
	startCol := l.column
	var sb strings.Builder
	for {
		c, ok := l.currentChar()
		if !ok || !(unicode.IsLetter(c) || unicode.IsDigit(c) || c == '_') {
			break
		}
		sb.WriteRune(l.advance())
	}
	val := sb.String()
	tType := TokenName
	if _, ok := l.keywordsSet[val]; ok {
		tType = TokenKeyword
	}
	return Token{Type: tType, Value: val, Line: startLine, Column: startCol}
}

func (l *Lexer) readString() Token {
	startLine := l.line
	startCol := l.column
	var sb strings.Builder
	l.advance() // Consume opening quote
	for {
		c, ok := l.currentChar()
		if !ok {
			panic(fmt.Sprintf("LexerError at %d:%d: Unterminated string literal", startLine, startCol))
		}
		if c == '"' {
			l.advance()
			break
		}
		if c == '\\' {
			l.advance()
			escaped, okEsc := l.currentChar()
			if !okEsc {
				panic(fmt.Sprintf("LexerError at %d:%d: Unterminated string literal (ends with backslash)", startLine, startCol))
			}
			switch escaped {
			case 'n':
				sb.WriteRune('\n')
			case 't':
				sb.WriteRune('\t')
			case '\\':
				sb.WriteRune('\\')
			case '"':
				sb.WriteRune('"')
			default:
				sb.WriteRune(escaped)
			}
			l.advance()
		} else {
			sb.WriteRune(c)
			l.advance()
		}
	}
	return Token{Type: TokenString, Value: sb.String(), Line: startLine, Column: startCol}
}

func (l *Lexer) Tokenize() []Token {
	l.tokens = []Token{}
	for {
		char, ok := l.currentChar()
		if !ok {
			break
		}
		if char == ' ' || char == '\t' || char == '\r' {
			l.skipWhitespace()
			continue
		}
		if char == '\n' {
			t := Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column}
			l.advance()
			l.tokens = append(l.tokens, t)
			continue
		}
		if unicode.IsDigit(char) {
			l.tokens = append(l.tokens, l.readNumber())
			continue
		}
		if unicode.IsLetter(char) || char == '_' {
			l.tokens = append(l.tokens, l.readName())
			continue
		}
		if char == '"' {
			l.tokens = append(l.tokens, l.readString())
			continue
		}
		// Lookahead required:
		if char == '=' {
			startLine := l.line
			startCol := l.column
			l.advance()
			if next, hasNext := l.currentChar(); hasNext && next == '=' {
				l.advance()
				l.tokens = append(l.tokens, Token{Type: TokenEqualsEquals, Value: "==", Line: startLine, Column: startCol})
			} else {
				l.tokens = append(l.tokens, Token{Type: TokenEquals, Value: "=", Line: startLine, Column: startCol})
			}
			continue
		}

		if tType, exists := SimpleTokens[char]; exists {
			t := Token{Type: tType, Value: string(char), Line: l.line, Column: l.column}
			l.advance()
			l.tokens = append(l.tokens, t)
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
	}
	l.tokens = append(l.tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column})
	return l.tokens
}
