package parser

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// A separator that is a sequence containing a rule reference can match the
// rule and then fail on the rest (`( r COMMA )` matching r, failing COMMA).
// Its partly matched children are dropped, so its height must not be counted.
// Here p is first built at depth 3 (inside w), then replayed from the memo one
// level deeper (through v): with an inflated height of 2 that replay would
// need depth 3 + 2 = 5 and be refused at MaxDepth 4, although the real tree
// (top > w > v > p) is exactly 4 levels tall.
func TestFailedSeparatorDoesNotInflateHeight(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
top = NUM r Z | w A ;
w = p X | v ;
v = p ;
p = { NUM // ( r COMMA ) } ;
r = A ;
`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{benchTok("NUM", "1"), benchTok("A", "a"), benchTok("EOF", "")}
	ast, err := c.Parse(toks, Options{MaxDepth: 4})
	if err != nil {
		t.Fatalf("a tree of exactly MaxDepth levels must parse: %v", err)
	}
	if h := treeHeight(ast); h != 4 {
		t.Fatalf("tree height %d, want 4", h)
	}
	// One level less is genuinely too deep.
	_, err = c.Parse(toks, Options{MaxDepth: 3})
	requireDepthError(t, err)
}
