package parser

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

type ParseError struct {
	Message string
	Token   lexer.Token
}

func (e *ParseError) Error() string {
	return fmt.Sprintf("%s at line %d, column %d", e.Message, e.Token.Line, e.Token.Column)
}

type Parser struct {
	tokens []lexer.Token
	pos    int
}

func NewParser(tokens []lexer.Token) *Parser {
	return &Parser{
		tokens: tokens,
		pos:    0,
	}
}

func (p *Parser) peek() lexer.Token {
	if p.pos < len(p.tokens) {
		return p.tokens[p.pos]
	}
	return p.tokens[len(p.tokens)-1]
}

func (p *Parser) advance() lexer.Token {
	token := p.peek()
	p.pos++
	return token
}

func (p *Parser) expect(expectedType lexer.TokenType) lexer.Token {
	token := p.peek()
	if token.Type != expectedType {
		panic(&ParseError{
			Message: fmt.Sprintf("Expected %s, got %s (%q)", expectedType, token.Type, token.Value),
			Token:   token,
		})
	}
	return p.advance()
}

func (p *Parser) match(types ...lexer.TokenType) *lexer.Token {
	token := p.peek()
	for _, t := range types {
		if token.Type == t {
			tok := p.advance()
			return &tok
		}
	}
	return nil
}

func (p *Parser) atEnd() bool {
	return p.peek().Type == lexer.TokenEOF
}

func (p *Parser) skipNewlines() {
	for p.peek().Type == lexer.TokenNewline {
		p.advance()
	}
}

func (p *Parser) Parse() Program {
	return p.parseProgram()
}

func (p *Parser) parseProgram() Program {
	var statements []Statement
	p.skipNewlines()
	for !p.atEnd() {
		stmt := p.parseStatement()
		statements = append(statements, stmt)
		p.skipNewlines()
	}
	return Program{Statements: statements}
}

func (p *Parser) parseStatement() Statement {
	if p.peek().Type == lexer.TokenName && p.pos+1 < len(p.tokens) && p.tokens[p.pos+1].Type == lexer.TokenEquals {
		return p.parseAssignment()
	}
	return p.parseExpressionStmt()
}

func (p *Parser) parseAssignment() Assignment {
	nameToken := p.expect(lexer.TokenName)
	target := Name{Name: nameToken.Value}

	p.expect(lexer.TokenEquals)

	value := p.parseExpression()

	if !p.atEnd() {
		p.expect(lexer.TokenNewline)
	}

	return Assignment{Target: target, Value: value}
}

func (p *Parser) parseExpressionStmt() ExpressionStmt {
	expr := p.parseExpression()
	if !p.atEnd() {
		p.expect(lexer.TokenNewline)
	}
	return ExpressionStmt{Expression: expr}
}

func (p *Parser) parseExpression() Expression {
	left := p.parseTerm()
	for {
		opTok := p.match(lexer.TokenPlus, lexer.TokenMinus)
		if opTok == nil {
			break
		}
		right := p.parseTerm()
		left = BinaryOp{Left: left, Op: opTok.Value, Right: right}
	}
	return left
}

func (p *Parser) parseTerm() Expression {
	left := p.parseFactor()
	for {
		opTok := p.match(lexer.TokenStar, lexer.TokenSlash)
		if opTok == nil {
			break
		}
		right := p.parseFactor()
		left = BinaryOp{Left: left, Op: opTok.Value, Right: right}
	}
	return left
}

func (p *Parser) parseFactor() Expression {
	token := p.peek()

	if token.Type == lexer.TokenNumber {
		p.advance()
		var val int
		fmt.Sscanf(token.Value, "%d", &val)
		return NumberLiteral{Value: val}
	}

	if token.Type == lexer.TokenString {
		p.advance()
		return StringLiteral{Value: token.Value}
	}

	if token.Type == lexer.TokenName {
		p.advance()
		return Name{Name: token.Value}
	}

	if token.Type == lexer.TokenLParen {
		p.advance()
		expr := p.parseExpression()
		p.expect(lexer.TokenRParen)
		return expr
	}

	panic(&ParseError{
		Message: fmt.Sprintf("Unexpected token %s (%q)", token.Type, token.Value),
		Token:   token,
	})
}
