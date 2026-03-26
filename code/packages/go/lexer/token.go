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
	TokenSemicolon
	TokenLBrace
	TokenRBrace
	TokenLBracket
	TokenRBracket
	TokenDot
	TokenBang
	TokenNewline
	TokenEOF
)

func (t TokenType) String() string {
	names := []string{
		"Name", "Number", "String", "Keyword",
		"Plus", "Minus", "Star", "Slash",
		"Equals", "EqualsEquals", "LParen", "RParen",
		"Comma", "Colon", "Semicolon", "LBrace", "RBrace",
		"LBracket", "RBracket", "Dot", "Bang", "Newline", "EOF",
	}
	if int(t) < len(names) {
		return names[t]
	}
	return "Unknown"
}

type Token struct {
	Type     TokenType
	Value    string
	Line     int
	Column   int
	TypeName string // Grammar-driven token name (e.g. "INT", "FLOAT"). Empty for hand-written lexer tokens.
}

// EffectiveTypeName returns the token type as a string. If TypeName is set
// (grammar-driven tokens with custom types like "SIZED_NUMBER"), it returns
// that. Otherwise it returns the TokenType enum name (e.g. "Name", "Number").
func (t Token) EffectiveTypeName() string {
	if t.TypeName != "" {
		return t.TypeName
	}
	return t.Type.String()
}

func (t Token) String() string {
	return fmt.Sprintf("Token(%s, %q, %d:%d)", t.Type, t.Value, t.Line, t.Column)
}
