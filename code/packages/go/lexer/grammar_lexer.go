package lexer

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

type compiledPattern struct {
	Name    string
	Pattern *regexp.Regexp
}

type GrammarLexer struct {
	source      string
	grammar     *grammartools.TokenGrammar
	pos         int
	line        int
	column      int
	keywordSet  map[string]struct{}
	patterns    []compiledPattern
}

func NewGrammarLexer(source string, grammar *grammartools.TokenGrammar) *GrammarLexer {
	keywordSet := make(map[string]struct{})
	for _, kw := range grammar.Keywords {
		keywordSet[kw] = struct{}{}
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
		patterns = append(patterns, compiledPattern{Name: defn.Name, Pattern: pat})
	}

	return &GrammarLexer{
		source:     source,
		grammar:    grammar,
		pos:        0,
		line:       1,
		column:     1,
		keywordSet: keywordSet,
		patterns:   patterns,
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

func (l *GrammarLexer) resolveTokenType(tokenName string, value string) TokenType {
	if tokenName == "NAME" {
		if _, ok := l.keywordSet[value]; ok {
			return TokenKeyword
		}
	}
	
	switch tokenName {
	case "NAME": return TokenName
	case "NUMBER": return TokenNumber
	case "STRING": return TokenString
	case "PLUS": return TokenPlus
	case "MINUS": return TokenMinus
	case "STAR": return TokenStar
	case "SLASH": return TokenSlash
	case "EQUALS": return TokenEquals
	case "EQUALS_EQUALS": return TokenEqualsEquals
	case "LPAREN": return TokenLParen
	case "RPAREN": return TokenRParen
	case "COMMA": return TokenComma
	case "COLON": return TokenColon
	case "SEMICOLON": return TokenSemicolon
	case "LBRACE": return TokenLBrace
	case "RBRACE": return TokenRBrace
	case "LBRACKET": return TokenLBracket
	case "RBRACKET": return TokenRBracket
	case "DOT": return TokenDot
	case "BANG": return TokenBang
	default:
		// Default to generic unmapped identifier bounding overlaps safely.
		return TokenName 
	}
}

func processEscapes(s string) string {
	var sb strings.Builder
	i := 0
	for i < len(s) {
		if s[i] == '\\' && i+1 < len(s) {
			next := s[i+1]
			switch next {
			case 'n': sb.WriteByte('\n')
			case 't': sb.WriteByte('\t')
			case '\\': sb.WriteByte('\\')
			case '"': sb.WriteByte('"')
			default: sb.WriteByte(next)
			}
			i += 2
		} else {
			sb.WriteByte(s[i])
			i++
		}
	}
	return sb.String()
}

func (l *GrammarLexer) Tokenize() []Token {
	var tokens []Token

	for l.pos < len(l.source) {
		char := l.source[l.pos]

		if char == ' ' || char == '\t' || char == '\r' {
			l.advance()
			continue
		}

		if char == '\n' {
			tokens = append(tokens, Token{Type: TokenNewline, Value: "\\n", Line: l.line, Column: l.column})
			l.advance()
			continue
		}

		remaining := l.source[l.pos:]
		matched := false

		for _, p := range l.patterns {
			loc := p.Pattern.FindStringIndex(remaining)
			if loc != nil && loc[0] == 0 {
				value := remaining[:loc[1]]
				startLine := l.line
				startCol := l.column
				
				tType := l.resolveTokenType(p.Name, value)

				if p.Name == "STRING" {
					inner := value[1 : len(value)-1]
					inner = processEscapes(inner)
					tokens = append(tokens, Token{Type: tType, Value: inner, Line: startLine, Column: startCol})
				} else {
					tokens = append(tokens, Token{Type: tType, Value: value, Line: startLine, Column: startCol})
				}

				for i := 0; i < len(value); i++ {
					l.advance()
				}

				matched = true
				break
			}
		}

		if !matched {
			panic(fmt.Sprintf("LexerError at %d:%d: Unexpected sequence %q...", l.line, l.column, char))
		}
	}

	tokens = append(tokens, Token{Type: TokenEOF, Value: "", Line: l.line, Column: l.column})
	return tokens
}
