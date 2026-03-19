package lexer

import "fmt"

type TokenType int

const (
	TokenName TokenType = iota
	TokenNumber
	TokenString
	TokenKeyword
	TokenPlus
	TokenMinus
	TokenStar
	TokenSlash
	TokenEquals
	TokenEqualsEquals
	TokenLParen
	TokenRParen
	TokenComma
	TokenColon
	TokenNewline
	TokenEOF
)

func (t TokenType) String() string {
	names := []string{
		"Name", "Number", "String", "Keyword",
		"Plus", "Minus", "Star", "Slash",
		"Equals", "EqualsEquals", "LParen", "RParen",
		"Comma", "Colon", "Newline", "EOF",
	}
	if int(t) < len(names) {
		return names[t]
	}
	return "Unknown"
}

type Token struct {
	Type   TokenType
	Value  string
	Line   int
	Column int
}

func (t Token) String() string {
	return fmt.Sprintf("Token(%s, %q, %d:%d)", t.Type, t.Value, t.Line, t.Column)
}
