package parser

// More engine coverage: repetition forms, enum tokens, end-of-input depth
// errors, and the AST walking helpers (which had no tests before).

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestSeparatedAndOneOrMoreRepetition(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
list = { NUMBER // COMMA }+ ;
maybe = { NAME // COMMA } ;
many = { NUMBER }+ ;
`))
	if err != nil {
		t.Fatal(err)
	}
	eof := benchTok("EOF", "")
	num, comma, name := benchTok("NUMBER", "1"), benchTok("COMMA", ","), benchTok("NAME", "a")

	ast, err := c.Parse([]lexer.Token{num, comma, num, comma, num, eof}, Options{StartRule: "list"})
	if err != nil || len(CollectTokens(ast, "")) != 5 {
		t.Fatalf("list: %v %v", ast, err)
	}
	// A dangling separator is not consumed, so the trailing COMMA is an error.
	if _, err := c.Parse([]lexer.Token{num, comma, eof}, Options{StartRule: "list"}); err == nil {
		t.Fatal("dangling separator must be rejected")
	}
	// at-least-one fails on empty input; the optional form accepts it.
	if _, err := c.Parse([]lexer.Token{eof}, Options{StartRule: "list"}); err == nil {
		t.Fatal("list needs one element")
	}
	if _, err := c.Parse([]lexer.Token{eof}, Options{StartRule: "maybe"}); err != nil {
		t.Fatalf("maybe accepts zero: %v", err)
	}
	if _, err := c.Parse([]lexer.Token{name, comma, name, eof}, Options{StartRule: "maybe"}); err != nil {
		t.Fatal(err)
	}
	ast, err = c.Parse([]lexer.Token{num, num, num, eof}, Options{StartRule: "many"})
	if err != nil || len(ast.Children) != 3 {
		t.Fatalf("many: %v %v", ast, err)
	}
	if _, err := c.Parse([]lexer.Token{name, eof}, Options{StartRule: "many"}); err == nil {
		t.Fatal("many needs one")
	}
}

func TestTrailingGarbageAndEnumTokens(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `program = NUMBER ;`))
	if err != nil {
		t.Fatal(err)
	}
	// Enum-typed tokens (no TypeName) still match by the legacy mapping.
	enum := []lexer.Token{{Type: lexer.TokenNumber, Value: "1"}, {Type: lexer.TokenEOF}}
	if _, err := c.Parse(enum, Options{}); err != nil {
		t.Fatalf("enum tokens: %v", err)
	}
	// Leftover tokens after a complete parse are an error.
	toks := []lexer.Token{benchTok("NUMBER", "1"), benchTok("NUMBER", "2"), benchTok("EOF", "")}
	if _, err := c.Parse(toks, Options{}); err == nil {
		t.Fatal("trailing token must be rejected")
	}
}

// A depth refusal that happens exactly at the end of the token list reports
// the synthetic EOF position instead of indexing out of range.
func TestDepthErrorAtEndOfTokens(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
a = NUMBER b ;
b = EOF ;
`))
	if err != nil {
		t.Fatal(err)
	}
	_, err = c.Parse([]lexer.Token{benchTok("NUMBER", "1")}, Options{MaxDepth: 1})
	requireDepthError(t, err)
}

func TestWalkFindCollect(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
list = LBRACKET [ item { COMMA item } ] RBRACKET ;
item = NUMBER ;
`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{
		benchTok("LBRACKET", "["), benchTok("NUMBER", "1"), benchTok("COMMA", ","),
		benchTok("NUMBER", "2"), benchTok("RBRACKET", "]"), benchTok("EOF", ""),
	}
	ast, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if n := len(FindNodes(ast, "item")); n != 2 {
		t.Fatalf("FindNodes = %d, want 2", n)
	}
	// Replace every item with an empty node via Leave; the parent is rebuilt.
	out := WalkAST(ast, ASTVisitor{
		Leave: func(n *ASTNode, _ *ASTNode) *ASTNode {
			if n.RuleName == "item" {
				return &ASTNode{RuleName: "item", Children: nil}
			}
			return nil
		},
	})
	if out == ast || len(CollectTokens(out, "NUMBER")) != 0 {
		t.Fatal("WalkAST should have produced a rewritten tree")
	}
	if same := WalkAST(ast, ASTVisitor{}); same != ast {
		t.Fatal("a no-op walk must return the original root")
	}
	rep := WalkAST(ast, ASTVisitor{Enter: func(n *ASTNode, p *ASTNode) *ASTNode {
		if p == nil {
			return &ASTNode{RuleName: "root", Children: n.Children}
		}
		return nil
	}})
	if rep.RuleName != "root" {
		t.Fatal("Enter should be able to replace the root")
	}
}
