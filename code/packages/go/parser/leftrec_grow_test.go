package parser

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// The rules the analysis marks really do grow: a left-recursive rule reached
// through a lookahead prefix still builds the left-leaning chain.
func TestLeftRecThroughLookaheadStillGrows(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, "a = !X a Y | Z ;"))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{
		{TypeName: "Z", Value: "z", Line: 1, Column: 1}, {TypeName: "Y", Value: "y", Line: 1, Column: 2},
		{TypeName: "Y", Value: "y", Line: 1, Column: 3}, {TypeName: "EOF", Line: 1, Column: 4},
	}
	ast, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	var sb strings.Builder
	dumpNew(ast, &sb)
	if want := "(a@1:1-1:3 nil=false(a@0:0-0:0 nil=false(a@0:0-0:0 nil=false Z:1) Y:2) Y:3)"; sb.String() != want {
		t.Fatalf("got %s", sb.String())
	}
}
