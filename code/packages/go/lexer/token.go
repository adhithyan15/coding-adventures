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

// ---------------------------------------------------------------------------
// Token Flag Constants
// ---------------------------------------------------------------------------

// Token flags carry metadata that is neither type nor value but affects how
// downstream consumers (parsers, formatters, linters) interpret a token.
// Flags are stored as a bitmask in Token.Flags. A zero value means no flags.
// Use bitwise AND to test: `token.Flags & TokenPrecededByNewline != 0`

// TokenPrecededByNewline is set when a line break appeared between this token
// and the previous one. Languages with automatic semicolon insertion
// (JavaScript, Go) use this to decide whether an implicit semicolon should
// be inserted. The lexer itself does not insert semicolons — that is a
// language-specific concern handled by post-tokenize hooks or parser hooks.
const TokenPrecededByNewline = 1

// TokenContextKeyword is set for context-sensitive keywords — words that are
// keywords in some syntactic positions but identifiers in others.
//
// For example, JavaScript's `async`, `yield`, `await`, `get`, `set` are
// sometimes keywords (in function declarations, property accessors) and
// sometimes plain identifiers (`let get = 5`). The lexer emits these as
// NAME tokens with this flag set, leaving the final keyword-vs-identifier
// decision to the language-specific parser.
const TokenContextKeyword = 2

type Token struct {
	Type     TokenType
	Value    string
	Line     int
	Column   int
	TypeName string // Grammar-driven token name (e.g. "INT", "FLOAT"). Empty for hand-written lexer tokens.
	Flags    int    // Bitmask of token flags (TokenPrecededByNewline, TokenContextKeyword). 0 means no flags.
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
