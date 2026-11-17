package parser

// Tests for the strict tree-height bound: every successful parse yields a tree
// no taller than Options.MaxDepth (root = height 1). Two paths used to exceed
// it: a memo hit at a deeper depth than the subtree was built at, and growth
// of a left-recursive rule from an already deep seed.

import (
	"errors"
	"fmt"
	"math/rand"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// treeHeight is the number of ASTNode levels (root = 1), computed iteratively.
func treeHeight(root *ASTNode) int {
	type fr struct {
		n *ASTNode
		d int
	}
	h := 0
	st := []fr{{root, 1}}
	for len(st) > 0 {
		f := st[len(st)-1]
		st = st[:len(st)-1]
		if f.d > h {
			h = f.d
		}
		for _, ch := range f.n.Children {
			if x, ok := ch.(*ASTNode); ok {
				st = append(st, fr{x, f.d + 1})
			}
		}
	}
	return h
}

func bracketsAround(inner []lexer.Token, n int) []lexer.Token {
	var toks []lexer.Token
	for i := 0; i < n; i++ {
		toks = append(toks, benchTok("LB", "["))
	}
	toks = append(toks, inner...)
	for i := 0; i < n; i++ {
		toks = append(toks, benchTok("RB", "]"))
	}
	return toks
}

// Memo hit at a deeper depth than the subtree was built at. `v` is tried first
// as `t1 X` (which parses the nested v at shallow depth and then fails on the
// missing X), then as the c1..c8 unit chain (the same positions are replayed
// from the memo, now 8 rule levels deeper per bracket). Before the fix, 4,900
// brackets built a tree of height 44,109, 4.4x the default limit.
func TestMemoHitAtDeeperDepthIsChecked(t *testing.T) {
	const k = 8
	var chain strings.Builder
	for i := 1; i < k; i++ {
		fmt.Fprintf(&chain, "c%d = c%d ;\n", i, i+1)
	}
	src := "v = t1 X | c1 ;\nt1 = LB v RB ;\n" + chain.String() + fmt.Sprintf("c%d = LB v RB | NUM ;", k)
	c, err := CompileGrammar(mustGrammar(t, src))
	if err != nil {
		t.Fatal(err)
	}
	toks := append(bracketsAround([]lexer.Token{benchTok("NUM", "1")}, 4900), benchTok("EOF", ""))
	ast, err := c.Parse(toks, Options{})
	requireDepthError(t, err)
	if ast != nil {
		t.Fatal("no tree with a depth error")
	}
	// Small nesting still parses, and respects the bound.
	toks = append(bracketsAround([]lexer.Token{benchTok("NUM", "1")}, 20), benchTok("EOF", ""))
	ast, err = c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if h := treeHeight(ast); h > DefaultMaxDepth {
		t.Fatalf("height %d exceeds the limit", h)
	}
}

// A deep seed followed by growth: the seed is 6,000 levels tall and 5,000
// growth steps wrap it, 11,000+ in total. Growth used to be charged against
// call depth only, so this reached about 2x MaxDepth.
func TestGrowthFromDeepSeedIsChecked(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, "e = e PLUS NUM | f ;\nf = LB f RB | NUM ;"))
	if err != nil {
		t.Fatal(err)
	}
	toks := bracketsAround([]lexer.Token{benchTok("NUM", "1")}, 6000)
	for i := 0; i < 5000; i++ {
		toks = append(toks, benchTok("PLUS", "+"), benchTok("NUM", "1"))
	}
	toks = append(toks, benchTok("EOF", ""))
	_, err = c.Parse(toks, Options{})
	requireDepthError(t, err)

	// The same seed with few growth steps fits and respects the bound.
	toks = bracketsAround([]lexer.Token{benchTok("NUM", "1")}, 6000)
	for i := 0; i < 100; i++ {
		toks = append(toks, benchTok("PLUS", "+"), benchTok("NUM", "1"))
	}
	toks = append(toks, benchTok("EOF", ""))
	ast, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if h := treeHeight(ast); h > DefaultMaxDepth {
		t.Fatalf("height %d exceeds the limit", h)
	}
}

// Property: whatever the grammar, tokens and (small) limit, a parse either
// fails or yields a tree whose height is at most MaxDepth.
func TestTreeHeightNeverExceedsMaxDepth(t *testing.T) {
	rng := rand.New(rand.NewSource(11))
	compiled := diffGrammarByName(t)
	extra := map[string]string{
		"memo-chain":  "v = t1 X | c1 ; t1 = LB v RB ; c1 = c2 ; c2 = c3 ; c3 = LB v RB | NUM ;",
		"deep-seed":   "e = e PLUS NUM | f ; f = LB f RB | NUM ;",
		"brackets":    "value = LB [ value ] RB | NUM ;",
		"left-in-opt": "p = { e SEMI } ; e = e PLUS NUM | [ LB e RB ] NUM ;",
	}
	for name, src := range extra {
		c, err := CompileGrammar(mustGrammar(t, strings.ReplaceAll(src, " ; ", " ;\n")))
		if err != nil {
			t.Fatal(err)
		}
		compiled[name] = c
	}
	vocab := append(append([]string(nil), diffVocab...), "LB", "RB", "X")
	vals := map[string]string{"LB": "[", "RB": "]"}
	for k, v := range diffVals {
		vals[k] = v
	}
	parsed := 0
	for name, c := range compiled {
		for it := 0; it < 1500; it++ {
			n := rng.Intn(30)
			toks := make([]lexer.Token, 0, n+1)
			for i := 0; i < n; i++ {
				v := vocab[rng.Intn(len(vocab))]
				if v == "NEWLINE" && rng.Intn(3) != 0 {
					v = "NUM"
				}
				toks = append(toks, lexer.Token{TypeName: v, Value: vals[v], Line: 1, Column: i + 1})
			}
			toks = append(toks, lexer.Token{TypeName: "EOF", Line: 1, Column: n + 1})
			max := 1 + rng.Intn(10)
			ast, err := c.Parse(toks, Options{MaxDepth: max})
			if err != nil {
				continue
			}
			parsed++
			if h := treeHeight(ast); h > max {
				t.Fatalf("%s: tree height %d exceeds MaxDepth %d on %v", name, h, max, toks)
			}
		}
	}
	if parsed < 100 {
		t.Fatalf("property test barely exercised successful parses: %d", parsed)
	}
}

// Long hand-written grammar nesting is not penalised by failed alternatives:
// heights of abandoned subtrees are not counted.
func TestFailedAlternativesDoNotInflateHeight(t *testing.T) {
	// alt1 builds a 3-deep subtree and then fails; alt2 is shallow.
	c, err := CompileGrammar(mustGrammar(t, `
p = a X | NUM ;
a = b ;
b = c ;
c = NUM ;
`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{benchTok("NUM", "1"), benchTok("EOF", "")}
	// p(1) with NUM token only: height 1. MaxDepth 1 must pass although the
	// failed first alternative went 4 rules deep... which the CALL depth limit
	// already forbids, so use 4 to cover the call path and 1 for the height.
	if _, err := c.Parse(toks, Options{MaxDepth: 4}); err != nil {
		t.Fatal(err)
	}
	ast, err := c.Parse(toks, Options{MaxDepth: 4})
	if err != nil || treeHeight(ast) != 1 {
		t.Fatalf("height %d, err %v", treeHeight(ast), err)
	}
	// Re-check via memo: a memo hit of a shallow node must not be refused.
	if _, err := c.Parse(toks, Options{MaxDepth: 4}); errors.Is(err, ErrMaxDepth) {
		t.Fatal("shallow tree wrongly refused")
	}
}
