package lexer

import (
	"fmt"
	"regexp"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

type compiledPattern struct {
	Name    string
	Pattern *regexp.Regexp
	Alias   string // Optional type alias
}

// GrammarLexer tokenizes source code using grammar-defined token patterns.
// Supports skip patterns, type aliases, reserved keywords, and indentation mode.
type GrammarLexer struct {
	source       string
	grammar      *grammartools.TokenGrammar
	pos          int
	line         int
	column       int
	keywordSet   map[string]struct{}
	reservedSet  map[string]struct{}
	patterns     []compiledPattern
	skipPatterns []*regexp.Regexp
	// Indentation mode state
	indentMode   bool
	indentStack  []int
	bracketDepth int
}

// NewGrammarLexer creates a new grammar-driven lexer.
func NewGrammarLexer(source string, grammar *grammartools.TokenGrammar) *GrammarLexer {
	keywordSet := make(map[string]struct{})
	for _, kw := range grammar.Keywords {
		keywordSet[kw] = struct{}{}
	}

	reservedSet := make(map[string]struct{})
	for _, rk := range grammar.ReservedKeywords {
		reservedSet[rk] = struct{}{}
	}

	var patterns []compiledPattern
	for _, defn := range grammar.Definitions {
		var patStr string
		if defn.IsRegex {
			patStr = "^" + defn.Pattern
		} else {
			patStr = "^" + regexp.QuoteMeta(defn.Pattern)
		}

		pat, err := regexp.Compile(patStr)
		if err != nil {
			panic(fmt.Sprintf("Failed to compile pattern for token %s: %v", defn.Name, err))
		}
		patterns = append(patterns, compiledPattern{Name: defn.Name, Pattern: pat, Alias: defn.Alias})
	}

	var skipPatterns []*regexp.Regexp
	for _, defn := range grammar.SkipDefinitions {
		var patStr string
		if defn.IsRegex {
			patStr = "^" + defn.Pattern
		} else {
			patStr = "^" + regexp.QuoteMeta(defn.Pattern)
		}
		pat, err := regexp.Compile(patStr)
		if err != nil {
			panic(fmt.Sprintf("Failed to compile skip pattern %s: %v", defn.Name, err))
		}
		skipPatterns = append(skipPatterns, pat)
	}

	return &GrammarLexer{
		source:       source,
		grammar:      grammar,
		pos:          0,
		line:         1,
		column:       1,
		keywordSet:   keywordSet,
		reservedSet:  reservedSet,
		patterns:     patterns,
		skipPatterns: skipPatterns,
		indentMode:   grammar.Mode == "indentation",
		indentStack:  []int{0},
		bracketDepth: 0,
	}
}

func (l *GrammarLexer) advance() {
	if l.pos < len(l.source) {
		if l.source[l.pos] == '\n' {
			l.line++
			l.column = 1
		} else {
			l.column++
		}
		l.pos++
	}
}

// resolveTokenType maps a grammar token name to a TokenType and TypeName.
// Returns (tokenType, typeName) where typeName is the effective name for
// the parser (alias if present, otherwise the definition name).
func (l *GrammarLexer) resolveTokenType(tokenName string, value string, alias string) (TokenType, string) {
	// Reserved keyword check
	if tokenName == "NAME" {
		if _, ok := l.reservedSet[value]; ok {
			panic(fmt.Sprintf("LexerError at %d:%d: Reserved keyword %q cannot be used as an identifier", l.line, l.column, value))
		}
	}

	// Regular keyword check
	if tokenName == "NAME" {
		if _, ok := l.keywordSet[value]; ok {
			return TokenKeyword, "KEYWORD"
		}
	}

	// Determine effective name (alias takes precedence)
	effectiveName := tokenName
	if alias != "" {
		effectiveName = alias
	}

	// Try known token types
	switch effectiveName {
	case "NAME":
		return TokenName, "NAME"
	case "NUMBER":
		return TokenNumber, "NUMBER"
	case "STRING":
		return TokenString, "STRING"
	case "PLUS":
		return TokenPlus, "PLUS"
	case "MINUS":
		return TokenMinus, "MINUS"
	case "STAR":
		return TokenStar, "STAR"
	case "SLASH":
		return TokenSlash, "SLASH"
	case "EQUALS":
		return TokenEquals, "EQUALS"
	case "EQUALS_EQUALS":
		return TokenEqualsEquals, "EQUALS_EQUALS"
	case "LPAREN":
		return TokenLParen, "LPAREN"
	case "RPAREN":
		return TokenRParen, "RPAREN"
	case "COMMA":
		return TokenComma, "COMMA"
	case "COLON":
		return TokenColon, "COLON"
	case "SEMICOLON":
		return TokenSemicolon, "SEMICOLON"
	case "LBRACE":
		return TokenLBrace, "LBRACE"
	case "RBRACE":
		return TokenRBrace, "RBRACE"
	case "LBRACKET":
		return TokenLBracket, "LBRACKET"
	case "RBRACKET":
		return TokenRBracket, "RBRACKET"
	case "DOT":
		return TokenDot, "DOT"
	case "BANG":
		return TokenBang, "BANG"
	default:
		// Unknown type -- use TokenName as base but store the string type
		return TokenName, effectiveName
	}
}

// trySkip attempts to match and consume a skip pattern at the current position.
func (l *GrammarLexer) trySkip() bool {
	remaining := l.source[l.pos:]
	for _, pat := range l.skipPatterns {
		loc := pat.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			for i := 0; i < loc[1]; i++ {
				l.advance()
			}
			return true
		}
	}
	return false
}

// tryMatchToken attempts to match a token at the current position.
func (l *GrammarLexer) tryMatchToken() *Token {
	remaining := l.source[l.pos:]

	for _, p := range l.patterns {
		loc := p.Pattern.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			value := remaining[:loc[1]]
			startLine := l.line
			startCol := l.column

			tType, typeName := l.resolveTokenType(p.Name, value, p.Alias)

			// Handle STRING tokens: strip quotes and process escapes.
			if p.Name == "STRING" || (p.Alias != "" && strings.Contains(p.Alias, "STRING")) {
				if len(value) >= 2 && (value[0] == '"' || value[0] == '\'') {
					inner := value[1 : len(value)-1]
					inner = processEscapes(inner)
					value = inner
				}
			}

			tok := Token{Type: tType, Value: value, Line: startLine, Column: startCol, TypeName: typeName}

			for i := 0; i < loc[1]; i++ {
				l.advance()
			}

			return &tok
		}
	}
	return nil
}

// processEscapes handles escape sequences in string literals.
func processEscapes(s string) string {
	var sb strings.Builder
	i := 0
	for i < len(s) {
		if s[i] == '\\' && i+1 < len(s) {
			next := s[i+1]
			switch next {
			case 'n':
				sb.WriteByte('\n')
			case 't':
				sb.WriteByte('\t')
			case '\\':
				sb.WriteByte('\\')
			case '"':
				sb.WriteByte('"')
			default:
				sb.WriteByte(next)
			}
			i += 2
		} else {
			sb.WriteByte(s[i])
			i++
		}
	}
	return sb.String()
}

// Tokenize tokenizes the source using the grammar's token definitions.
func (l *GrammarLexer) Tokenize() []Token {
	if l.indentMode {
		return l.tokenizeIndentation()
	}
	return l.tokenizeStandard()
}

func (l *GrammarLexer) tokenizeStandard() []Token {
	var tokens []Token

	for l.pos < len(l.source) {
		char := l.source[l.pos]

		if char == ' ' || char == '\t' || char == '\r' {
			l.advance()
			continue
		}

		if char == '\n' {
			tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
			l.advance()
			continue
		}

		if l.trySkip() {
			continue
		}

		tok := l.tryMatchToken()
		if tok != nil {
			tokens = append(tokens, *tok)
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})
	return tokens
}

func (l *GrammarLexer) tokenizeIndentation() []Token {
	var tokens []Token
	atLineStart := true

	for l.pos < len(l.source) {
		// Process line start
		if atLineStart && l.bracketDepth == 0 {
			indentTokens, skipLine := l.processLineStart()
			if skipLine {
				continue
			}
			tokens = append(tokens, indentTokens...)
			atLineStart = false
			if l.pos >= len(l.source) {
				break
			}
		}

		char := l.source[l.pos]

		// Newline handling
		if char == '\n' {
			if l.bracketDepth == 0 {
				tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
			}
			l.advance()
			atLineStart = true
			continue
		}

		// Inside brackets: skip whitespace
		if l.bracketDepth > 0 && (char == ' ' || char == '\t' || char == '\r') {
			l.advance()
			continue
		}

		// Try skip patterns
		if l.trySkip() {
			continue
		}

		// Try token patterns
		tok := l.tryMatchToken()
		if tok != nil {
			// Track bracket depth
			switch tok.Value {
			case "(", "[", "{":
				l.bracketDepth++
			case ")", "]", "}":
				l.bracketDepth--
			}
			tokens = append(tokens, *tok)
			continue
		}

		panic(fmt.Sprintf("LexerError at %d:%d: Unexpected character %q", l.line, l.column, char))
	}

	// EOF: emit remaining DEDENTs
	for len(l.indentStack) > 1 {
		l.indentStack = l.indentStack[:len(l.indentStack)-1]
		tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: l.column, TypeName: "DEDENT"})
	}

	// Final NEWLINE if needed
	if len(tokens) == 0 || tokens[len(tokens)-1].Type != TokenNewline {
		tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column, TypeName: "NEWLINE"})
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column, TypeName: "EOF"})
	return tokens
}

// processLineStart handles indentation at the start of a logical line.
// Returns (indentTokens, skipLine).
func (l *GrammarLexer) processLineStart() ([]Token, bool) {
	indent := 0
	for l.pos < len(l.source) {
		char := l.source[l.pos]
		if char == ' ' {
			indent++
			l.advance()
		} else if char == '\t' {
			panic(fmt.Sprintf("LexerError at %d:%d: Tab character in indentation (use spaces only)", l.line, l.column))
		} else {
			break
		}
	}

	// Blank line or EOF
	if l.pos >= len(l.source) {
		return nil, true
	}
	if l.source[l.pos] == '\n' {
		l.advance()
		return nil, true
	}

	// Comment-only line
	remaining := l.source[l.pos:]
	for _, pat := range l.skipPatterns {
		loc := pat.FindStringIndex(remaining)
		if loc != nil && loc[0] == 0 {
			peekPos := l.pos + loc[1]
			if peekPos >= len(l.source) || l.source[peekPos] == '\n' {
				for i := 0; i < loc[1]; i++ {
					l.advance()
				}
				if l.pos < len(l.source) && l.source[l.pos] == '\n' {
					l.advance()
				}
				return nil, true
			}
		}
	}

	// Compare indent to current level
	currentIndent := l.indentStack[len(l.indentStack)-1]
	var tokens []Token

	if indent > currentIndent {
		l.indentStack = append(l.indentStack, indent)
		tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: 1, TypeName: "INDENT"})
	} else if indent < currentIndent {
		for len(l.indentStack) > 1 && l.indentStack[len(l.indentStack)-1] > indent {
			l.indentStack = l.indentStack[:len(l.indentStack)-1]
			tokens = append(tokens, Token{Type: TokenName, Value: "", Line: l.line, Column: 1, TypeName: "DEDENT"})
		}
		if l.indentStack[len(l.indentStack)-1] != indent {
			panic(fmt.Sprintf("LexerError at %d:%d: Inconsistent dedent", l.line, l.column))
		}
	}

	return tokens, false
}
