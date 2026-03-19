package parser

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

type ASTNode struct {
	RuleName string
	Children []interface{} // Can be *ASTNode or lexer.Token
}

func (n *ASTNode) IsLeaf() bool {
	if len(n.Children) == 1 {
		_, ok := n.Children[0].(lexer.Token)
		return ok
	}
	return false
}

func (n *ASTNode) Token() *lexer.Token {
	if n.IsLeaf() {
		tok := n.Children[0].(lexer.Token)
		return &tok
	}
	return nil
}

type GrammarParseError struct {
	Message string
	Token   lexer.Token
}

func (e *GrammarParseError) Error() string {
	return fmt.Sprintf("Parse error at %d:%d: %s", e.Token.Line, e.Token.Column, e.Message)
}

type GrammarParser struct {
	tokens  []lexer.Token
	grammar *grammartools.ParserGrammar
	pos     int
	rules   map[string]grammartools.GrammarRule
}

func NewGrammarParser(tokens []lexer.Token, grammar *grammartools.ParserGrammar) *GrammarParser {
	rules := make(map[string]grammartools.GrammarRule)
	for _, rule := range grammar.Rules {
		rules[rule.Name] = rule
	}
	return &GrammarParser{
		tokens:  tokens,
		grammar: grammar,
		pos:     0,
		rules:   rules,
	}
}

func (p *GrammarParser) current() lexer.Token {
	if p.pos < len(p.tokens) {
		return p.tokens[p.pos]
	}
	return p.tokens[len(p.tokens)-1]
}

func (p *GrammarParser) Parse() (*ASTNode, error) {
	if len(p.grammar.Rules) == 0 {
		return nil, fmt.Errorf("Grammar has no rules")
	}

	entryRule := p.grammar.Rules[0]
	result := p.parseRule(entryRule.Name)

	if result == nil {
		tok := p.current()
		return nil, &GrammarParseError{
			Message: fmt.Sprintf("Failed to parse grammar structurally safely starting across entry bounds."),
			Token:   tok,
		}
	}

	for p.pos < len(p.tokens) && p.current().Type == lexer.TokenNewline {
		p.pos++
	}

	if p.pos < len(p.tokens) && p.current().Type != lexer.TokenEOF {
		return nil, &GrammarParseError{
			Message: fmt.Sprintf("Unexpected token: %q", p.current().Value),
			Token:   p.current(),
		}
	}

	return result, nil
}

func (p *GrammarParser) parseRule(ruleName string) *ASTNode {
	rule, exists := p.rules[ruleName]
	if !exists {
		return nil
	}

	children, ok := p.matchElement(rule.Body)
	if !ok || children == nil {
		return nil
	}

	return &ASTNode{RuleName: ruleName, Children: children}
}

func stringToTokenType(id string) lexer.TokenType {
	switch id {
	case "NAME": return lexer.TokenName
	case "NUMBER": return lexer.TokenNumber
	case "STRING": return lexer.TokenString
	case "KEYWORD": return lexer.TokenKeyword
	case "PLUS": return lexer.TokenPlus
	case "MINUS": return lexer.TokenMinus
	case "STAR": return lexer.TokenStar
	case "SLASH": return lexer.TokenSlash
	case "EQUALS": return lexer.TokenEquals
	case "EQUALS_EQUALS": return lexer.TokenEqualsEquals
	case "LPAREN": return lexer.TokenLParen
	case "RPAREN": return lexer.TokenRParen
	case "COMMA": return lexer.TokenComma
	case "COLON": return lexer.TokenColon
	case "NEWLINE": return lexer.TokenNewline
	case "EOF": return lexer.TokenEOF
	// JavaScript/TypeScript delimiter tokens
	case "SEMICOLON": return lexer.TokenSemicolon
	case "LBRACE": return lexer.TokenLBrace
	case "RBRACE": return lexer.TokenRBrace
	case "LBRACKET": return lexer.TokenLBracket
	case "RBRACKET": return lexer.TokenRBracket
	case "DOT": return lexer.TokenDot
	case "BANG": return lexer.TokenBang
	default: return lexer.TokenName
	}
}

func (p *GrammarParser) matchElement(element grammartools.GrammarElement) ([]interface{}, bool) {
	savePos := p.pos

	switch e := element.(type) {
	case grammartools.Sequence:
		var children []interface{}
		for _, sub := range e.Elements {
			res, ok := p.matchElement(sub)
			if !ok {
				p.pos = savePos
				return nil, false
			}
			children = append(children, res...)
		}
		return children, true

	case grammartools.Alternation:
		for _, choice := range e.Choices {
			p.pos = savePos
			res, ok := p.matchElement(choice)
			if ok {
				return res, true
			}
		}
		p.pos = savePos
		return nil, false

	case grammartools.Repetition:
		var children []interface{}
		for {
			saveRep := p.pos
			res, ok := p.matchElement(e.Element)
			if !ok {
				p.pos = saveRep
				break
			}
			children = append(children, res...)
		}
		return children, true

	case grammartools.Optional:
		res, ok := p.matchElement(e.Element)
		if !ok {
			return []interface{}{}, true
		}
		return res, true

	case grammartools.Group:
		return p.matchElement(e.Element)

	case grammartools.RuleReference:
		if e.IsToken {
			token := p.current()
			for token.Type == lexer.TokenNewline && e.Name != "NEWLINE" {
				p.pos++
				token = p.current()
			}

			expectedType := stringToTokenType(e.Name)
			if token.Type == expectedType {
				p.pos++
				return []interface{}{token}, true
			}
			return nil, false
		} else {
			node := p.parseRule(e.Name)
			if node != nil {
				return []interface{}{node}, true
			}
			p.pos = savePos
			return nil, false
		}

	case grammartools.Literal:
		token := p.current()
		if token.Value == e.Value {
			p.pos++
			return []interface{}{token}, true
		}
		return nil, false
	}

	return nil, false
}
