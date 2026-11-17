package parser

// Tests for the flat-tree engine core (see "Parse state, and the FLAT tree
// representation" in compiled.go): the legacy-visible node semantics it must
// preserve (memo-hit nodes, empty Children), the cap-clipped children slab,
// the pooled buffers, and int32 index overflow.

import (
	"errors"
	"math/rand"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	old "github.com/adhithyan15/coding-adventures/code/packages/go/parser/internal/oldengine"
)

func dumpString(n *ASTNode) string {
	var sb strings.Builder
	canonDump(&sb, n)
	return sb.String()
}

func mustCompile(t testing.TB, src string) *Compiled {
	t.Helper()
	c, err := CompileGrammar(mustGrammar(t, src))
	if err != nil {
		t.Fatal(err)
	}
	return c
}

func tokAt(typ, val string, line, col int) lexer.Token {
	return lexer.Token{TypeName: typ, Value: val, Line: line, Column: col}
}

// A memo hit is a DISTINCT node that SHARES its source's Children slice and
// keeps zero positions. `x = EOF` matches the synthetic EOF without moving, so
// the second `x` at the same place is a memo hit with non-empty children, and
// both nodes are in the finished tree.
func TestMemoHitNodesAreDistinctAndShareChildren(t *testing.T) {
	c := mustCompile(t, `p = A x x ; x = EOF ;`)
	ast, err := c.Parse([]lexer.Token{tokAt("A", "a", 3, 7)}, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if len(ast.Children) != 3 {
		t.Fatalf("root children: %d", len(ast.Children))
	}
	x1, ok1 := ast.Children[1].(*ASTNode)
	x2, ok2 := ast.Children[2].(*ASTNode)
	if !ok1 || !ok2 {
		t.Fatalf("children are %T and %T", ast.Children[1], ast.Children[2])
	}
	if x1 == x2 {
		t.Fatal("the memo-hit node must be a distinct *ASTNode")
	}
	if len(x1.Children) != 1 || len(x2.Children) != 1 || &x1.Children[0] != &x2.Children[0] {
		t.Fatal("the memo-hit node must share the source node's Children backing array")
	}
	// The synthetic EOF carries the last token's line and column.
	eof, ok := x1.Children[0].(lexer.Token)
	if !ok || eof.TypeName != "EOF" || eof.Line != 3 || eof.Column != 7 {
		t.Fatalf("synthetic EOF: %#v", x1.Children[0])
	}
	if x1.StartLine != 3 || x1.StartColumn != 7 || x1.EndLine != 3 || x1.EndColumn != 7 {
		t.Fatalf("the parsed node has positions: %+v", x1)
	}
	if x2.StartLine != 0 || x2.StartColumn != 0 || x2.EndLine != 0 || x2.EndColumn != 0 {
		t.Fatalf("the memo-hit node must keep zero positions: %+v", x2)
	}
	// The parent still gets a position from the last leaf token under it.
	if ast.EndLine != 3 || ast.EndColumn != 7 {
		t.Fatalf("parent end position: %+v", ast)
	}
}

// A memo hit whose source node was DISCARDED by a lookahead: the source is
// not in the tree, but the hit is, and its children must still be built.
func TestMemoHitOfDiscardedSource(t *testing.T) {
	c := mustCompile(t, `p = &x x ; x = A B ;`)
	ast, err := c.Parse([]lexer.Token{tokAt("A", "a", 1, 1), tokAt("B", "b", 1, 2), tokAt("EOF", "", 1, 3)}, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if len(ast.Children) != 1 {
		t.Fatalf("root children: %d", len(ast.Children))
	}
	x := ast.Children[0].(*ASTNode)
	if x.RuleName != "x" || len(x.Children) != 2 || x.StartLine != 0 {
		t.Fatalf("memo-hit node: %+v", x)
	}
	if tk := x.Children[1].(lexer.Token); tk.TypeName != "B" || tk.Column != 2 {
		t.Fatalf("second child: %+v", tk)
	}
	// The root is parsed (not a hit), so it has positions computed from its
	// zero-position child by descending to the tokens.
	if ast.StartLine != 1 || ast.StartColumn != 1 || ast.EndColumn != 2 {
		t.Fatalf("root positions: %+v", ast)
	}
}

// Empty Children are always the non-nil empty slice, including a memo hit of
// an empty rule and an empty repetition.
func TestEmptyChildrenAreNonNil(t *testing.T) {
	c := mustCompile(t, `p = x x r NUM ; x = [ X ] ; r = { Y } ;`)
	ast, err := c.Parse([]lexer.Token{tokAt("NUM", "1", 1, 1)}, Options{})
	if err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 3; i++ {
		n := ast.Children[i].(*ASTNode)
		if n.Children == nil || len(n.Children) != 0 {
			t.Fatalf("child %d (%s) Children = %#v", i, n.RuleName, n.Children)
		}
	}
	if n := ast.Children[1].(*ASTNode); n.StartLine != 0 {
		t.Fatalf("empty memo hit has positions: %+v", n)
	}
}

// collectNodes lists every distinct *ASTNode reachable from root, iteratively.
func collectNodes(root *ASTNode) []*ASTNode {
	seen := map[*ASTNode]bool{root: true}
	all := []*ASTNode{root}
	for i := 0; i < len(all); i++ {
		for _, c := range all[i].Children {
			if n, ok := c.(*ASTNode); ok && !seen[n] {
				seen[n] = true
				all = append(all, n)
			}
		}
	}
	return all
}

// Every Children slice is cap-clipped to its length, so a consumer's append
// reallocates instead of overwriting the next sibling's slot in the shared
// slab; and appending to every node changes nothing anywhere else.
func TestChildrenAreCapClipped(t *testing.T) {
	for _, tc := range []struct {
		name, grammar string
		toks          []lexer.Token
	}{
		{"json-like", jsonLikeGrammar, jsonLikeTokens(5)},
		{"expression", exprGrammar, exprTokens(6)},
		{"eof-memo", `p = A x x ; x = EOF ;`, []lexer.Token{tokAt("A", "a", 1, 1)}},
	} {
		t.Run(tc.name, func(t *testing.T) {
			c := mustCompile(t, tc.grammar)
			ast, err := c.Parse(tc.toks, Options{})
			if err != nil {
				t.Fatal(err)
			}
			before := dumpString(ast)
			all := collectNodes(ast)
			if len(all) < 3 {
				t.Fatalf("only %d nodes", len(all))
			}
			for _, n := range all {
				if cap(n.Children) != len(n.Children) {
					t.Fatalf("%s: cap %d != len %d", n.RuleName, cap(n.Children), len(n.Children))
				}
			}
			orig := make([][]interface{}, len(all))
			for i, n := range all {
				orig[i] = n.Children
				n.Children = append(n.Children, lexer.Token{TypeName: "INTRUDER"})
			}
			// Put the original slice headers back. If any append had
			// written through into a shared slab, the originals (which a
			// sibling or the same node still sees) would now differ.
			for i, n := range all {
				n.Children = orig[i]
			}
			if after := dumpString(ast); after != before {
				t.Fatal("an append on one node's Children changed another node")
			}
		})
	}
}

// The shared-Children structure and positions match the frozen original
// engine, not just the PR 1 engine's goldens: same canonical dump (with the
// sharing markers) over every differential grammar and random streams.
func dumpOldShared(n *old.ASTNode, sb *strings.Builder, seen map[*interface{}]int) {
	sb.WriteString("(" + n.RuleName)
	sb.WriteString(" " + itoa(n.StartLine) + ":" + itoa(n.StartColumn) + "-" + itoa(n.EndLine) + ":" + itoa(n.EndColumn))
	if len(n.Children) == 0 {
		sb.WriteString(")")
		return
	}
	p := &n.Children[0]
	if k, dup := seen[p]; dup {
		sb.WriteString(" =" + itoa(k) + ")")
		return
	}
	seen[p] = len(seen)
	sb.WriteString(" #" + itoa(seen[p]))
	for _, c := range n.Children {
		switch x := c.(type) {
		case *old.ASTNode:
			sb.WriteString(" ")
			dumpOldShared(x, sb, seen)
		case lexer.Token:
			sb.WriteString(" [" + x.TypeName + ":" + itoa(x.Column) + "]")
		}
	}
	sb.WriteString(")")
}

func dumpNewShared(n *ASTNode, sb *strings.Builder, seen map[*interface{}]int) {
	sb.WriteString("(" + n.RuleName)
	sb.WriteString(" " + itoa(n.StartLine) + ":" + itoa(n.StartColumn) + "-" + itoa(n.EndLine) + ":" + itoa(n.EndColumn))
	if len(n.Children) == 0 {
		sb.WriteString(")")
		return
	}
	p := &n.Children[0]
	if k, dup := seen[p]; dup {
		sb.WriteString(" =" + itoa(k) + ")")
		return
	}
	seen[p] = len(seen)
	sb.WriteString(" #" + itoa(seen[p]))
	for _, c := range n.Children {
		switch x := c.(type) {
		case *ASTNode:
			sb.WriteString(" ")
			dumpNewShared(x, sb, seen)
		case lexer.Token:
			sb.WriteString(" [" + x.TypeName + ":" + itoa(x.Column) + "]")
		}
	}
	sb.WriteString(")")
}

func itoa(i int) string {
	if i == 0 {
		return "0"
	}
	neg := i < 0
	if neg {
		i = -i
	}
	var b []byte
	for ; i > 0; i /= 10 {
		b = append([]byte{byte('0' + i%10)}, b...)
	}
	if neg {
		b = append([]byte{'-'}, b...)
	}
	return string(b)
}

func TestSharingDifferentialAgainstFrozenOldEngine(t *testing.T) {
	n := 1500
	if testing.Short() {
		n = 150
	}
	rng := rand.New(rand.NewSource(11))
	compiled := diffGrammarByName(t)
	for gi, g := range diffGrammars {
		pg := mustGrammar(t, strings.ReplaceAll(g.src, " ; ", " ;\n"))
		for it := 0; it < n; it++ {
			toks := diffTokens(rng, rng.Intn(14))
			oa, oe := old.NewGrammarParser(append([]lexer.Token(nil), toks...), pg).Parse()
			na, ne := compiled[diffGrammars[gi].name].Parse(toks, Options{})
			if (oe == nil) != (ne == nil) {
				t.Fatalf("%s: old err %v, new err %v", g.name, oe, ne)
			}
			if oe != nil {
				continue
			}
			var osb, nsb strings.Builder
			dumpOldShared(oa, &osb, map[*interface{}]int{})
			dumpNewShared(na, &nsb, map[*interface{}]int{})
			if osb.String() != nsb.String() {
				t.Fatalf("%s: sharing/positions differ\n old %s\n new %s", g.name, osb.String(), nsb.String())
			}
		}
	}
}

// Whatever the previous parse did (succeeded, hit a syntax error, a depth,
// memo or token limit), the next parse on the same Compiled produces exactly
// what a brand-new Compiled produces.
func TestPoolReuseIsCleanAfterEveryOutcome(t *testing.T) {
	good := jsonLikeTokens(30)
	small := jsonLikeTokens(2)
	bad := append(append([]lexer.Token(nil), good[:len(good)/2]...), benchTok("EOF", ""))
	for _, opts := range []Options{{}, {MaxDepth: 4}, {MaxMemoSlots: 40}, {MaxTokens: 5}} {
		c := mustCompile(t, jsonLikeGrammar)
		for round := 0; round < 3; round++ {
			for _, toks := range [][]lexer.Token{good, bad, small, good} {
				got, gerr := c.Parse(toks, opts)
				want, werr := mustCompile(t, jsonLikeGrammar).Parse(toks, opts)
				if (gerr == nil) != (werr == nil) || (gerr != nil && gerr.Error() != werr.Error()) {
					t.Fatalf("opts %+v: err %v vs fresh %v", opts, gerr, werr)
				}
				if gerr == nil && dumpString(got) != dumpString(want) {
					t.Fatalf("opts %+v: reused state produced a different tree", opts)
				}
				if gerr != nil && got != nil {
					t.Fatal("tree returned with an error")
				}
			}
		}
	}
}

// A parse that panics must not hand its half-mutated state back to the pool,
// and must not poison later parses.
func TestPanicNeverReturnsStateToPool(t *testing.T) {
	c := mustCompile(t, `p = X q | Y ; q = Z ;`)
	// Seed the pool with a recognisable state.
	marker := c.pool.Get().(*parseState)
	marker.stack = make([]int32, 0, 4242)
	c.pool.Put(marker)
	// Break rule q: a reference to a rule id that does not exist in the table
	// makes the matcher index out of range, which Parse must turn into an error.
	c.rules[1].body = &node{kind: kRule, rule: 99}
	_, err := c.Parse([]lexer.Token{tokAt("X", "x", 1, 1), tokAt("Z", "z", 1, 2)}, Options{})
	if err == nil || !strings.Contains(err.Error(), "internal error") {
		t.Fatalf("want an internal error, got %v", err)
	}
	for i := 0; i < 50; i++ {
		got := c.pool.Get().(*parseState)
		if got == marker {
			t.Fatal("the state of a panicking parse was returned to the pool")
		}
		c.pool.Put(got)
	}
	// Later parses of unaffected input are fine and clean.
	ast, err := c.Parse([]lexer.Token{tokAt("Y", "y", 1, 1)}, Options{})
	if err != nil || dumpString(ast) != "(p@1:1-1:1 #0 [Y:\"y\":1:1])" {
		t.Fatalf("after a panic: %v %v", ast, err)
	}
}

// An idle pooled state holds no pointers to the caller's tokens or the
// finished tree, and the pooled buffers are bounded.
func TestReleaseClearsScratchAndBoundsBuffers(t *testing.T) {
	c := mustCompile(t, jsonLikeGrammar)
	toks := jsonLikeTokens(10)
	s := c.pool.Get().(*parseState)
	s.reset(toks, Options{})
	if _, err := s.run(0); err != nil {
		t.Fatal(err)
	}
	used := 0
	for _, b := range s.boxed {
		if b != nil {
			used++
		}
	}
	for _, p := range s.ptrs {
		if p != nil {
			used++
		}
	}
	if used == 0 {
		t.Fatal("toAST left no scratch to clear; the check below proves nothing")
	}
	boxed, ptrs := s.boxed, s.ptrs
	s.release()
	for i := range boxed {
		if boxed[i] != nil {
			t.Fatal("release left a boxed token in the pooled state")
		}
	}
	for i := range ptrs {
		if ptrs[i] != nil {
			t.Fatal("release left an AST node in the pooled state")
		}
	}

	// Oversized buffers are dropped, not pooled.
	s = c.pool.Get().(*parseState)
	s.reset(toks, Options{})
	s.stack = make([]int32, 0, maxPooledSlots+1)
	s.kids = make([]int32, 0, maxPooledSlots+1)
	s.nodes = make([]cnode, 0, maxPooledSlots+1)
	s.boxed = make([]interface{}, 0, maxPooledSlots+1)
	s.ptrs = make([]*ASTNode, 0, maxPooledSlots+1)
	s.mark = make([]uint8, 0, maxPooledSlots+1)
	s.coff = make([]int32, 0, maxPooledSlots+1)
	s.release()
	if s.stack != nil || s.kids != nil || s.nodes != nil || s.boxed != nil || s.ptrs != nil || s.mark != nil || s.coff != nil {
		t.Fatal("an oversized buffer was kept for reuse")
	}
}

// Index overflow fails closed: ErrMaxMemoSlots (or ErrMaxTokens for the token
// list), a nil tree, and a clean state afterwards, whichever of the three
// growth sites (kids, nodes, memo-hit nodes) runs out first.
func TestFlatIndexOverflowFailsClosed(t *testing.T) {
	defer func(l int) { flatIndexLimit = l }(flatIndexLimit)
	c := mustCompile(t, `p = &x x y ; x = A B ; y = { z } ; z = C ;`)
	toks := []lexer.Token{tokAt("A", "a", 1, 1), tokAt("B", "b", 1, 2)}
	for i := 0; i < 10; i++ {
		toks = append(toks, tokAt("C", "c", 1, 3+i))
	}
	toks = append(toks, tokAt("EOF", "", 1, 13))
	flatIndexLimit = 1 << 30
	want, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	wantDump := dumpString(want)
	sawMemo, sawTokens, sawOK := false, false, false
	for limit := 0; limit < 60; limit++ {
		flatIndexLimit = limit
		ast, err := c.Parse(toks, Options{})
		switch {
		case err == nil:
			sawOK = true
			if dumpString(ast) != wantDump {
				t.Fatalf("limit %d: wrong tree", limit)
			}
		case errors.Is(err, ErrMaxMemoSlots):
			sawMemo = true
			if ast != nil {
				t.Fatalf("limit %d: tree returned with %v", limit, err)
			}
			if !strings.Contains(err.Error(), "tree storage index space") {
				t.Fatalf("limit %d: message %q", limit, err)
			}
		case errors.Is(err, ErrMaxTokens):
			sawTokens = true
			if ast != nil {
				t.Fatalf("limit %d: tree returned with %v", limit, err)
			}
		default:
			t.Fatalf("limit %d: unexpected error %v", limit, err)
		}
	}
	if !sawMemo || !sawTokens || !sawOK {
		t.Fatalf("sweep did not cover every outcome: memo=%v tokens=%v ok=%v", sawMemo, sawTokens, sawOK)
	}
	flatIndexLimit = 1 << 30
	if got, err := c.Parse(toks, Options{}); err != nil || dumpString(got) != wantDump {
		t.Fatalf("state poisoned by overflow: %v", err)
	}

	// Memo-hit nodes are not kids of anything (a lookahead discards them), so
	// a parse with many of them runs out of NODE index space while the kids
	// array is nearly empty: the third growth site.
	c2 := mustCompile(t, `p = &x &x &x &x &x &x &x &x x ; x = A ;`)
	toks2 := []lexer.Token{tokAt("A", "a", 1, 1), tokAt("EOF", "", 1, 2)}
	flatIndexLimit = 1 << 30
	want2, err := c2.Parse(toks2, Options{})
	if err != nil {
		t.Fatal(err)
	}
	overflowed := false
	for limit := 3; limit < 14; limit++ {
		flatIndexLimit = limit
		ast, err := c2.Parse(toks2, Options{})
		if err == nil {
			if dumpString(ast) != dumpString(want2) {
				t.Fatalf("limit %d: wrong tree", limit)
			}
			continue
		}
		if !errors.Is(err, ErrMaxMemoSlots) || ast != nil {
			t.Fatalf("limit %d: %v", limit, err)
		}
		overflowed = true
	}
	if !overflowed {
		t.Fatal("no memo-node overflow")
	}

	// ... and inside left-recursion growth, where each step seals a new
	// result: every limit either succeeds with the right tree or fails closed.
	c3 := mustCompile(t, `e = e PLUS NUM | NUM ;`)
	toks3 := []lexer.Token{tokAt("NUM", "1", 1, 1)}
	for i := 0; i < 8; i++ {
		toks3 = append(toks3, tokAt("PLUS", "+", 1, 2+2*i), tokAt("NUM", "1", 1, 3+2*i))
	}
	toks3 = append(toks3, tokAt("EOF", "", 1, 20))
	flatIndexLimit = 1 << 30
	want3, err := c3.Parse(toks3, Options{})
	if err != nil {
		t.Fatal(err)
	}
	for limit := 19; limit < 80; limit++ {
		flatIndexLimit = limit
		ast, err := c3.Parse(toks3, Options{})
		if err == nil {
			if dumpString(ast) != dumpString(want3) {
				t.Fatalf("limit %d: wrong tree", limit)
			}
			continue
		}
		if !errors.Is(err, ErrMaxMemoSlots) || ast != nil {
			t.Fatalf("limit %d: %v", limit, err)
		}
	}
}

// The memo-hit growth path (left recursion) keeps node identity right: the
// seed that growth supersedes is replayed by a hit, which shares the seed's
// children, and the final node's children are the last growth step's.
func TestLeftRecursiveGrowthNodes(t *testing.T) {
	c := mustCompile(t, `e = e PLUS NUM | NUM ;`)
	toks := []lexer.Token{tokAt("NUM", "1", 1, 1), tokAt("PLUS", "+", 1, 2), tokAt("NUM", "2", 1, 3),
		tokAt("PLUS", "+", 1, 4), tokAt("NUM", "3", 1, 5), tokAt("EOF", "", 1, 6)}
	ast, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	// ((1 + 2) + 3): the root's first child is the previous result, replayed
	// from the memo: a node with no positions of its own.
	inner := ast.Children[0].(*ASTNode)
	if len(ast.Children) != 3 || inner.RuleName != "e" || len(inner.Children) != 3 {
		t.Fatalf("shape: %s", dumpString(ast))
	}
	if inner.StartLine != 0 {
		t.Fatalf("replayed growth seed has positions: %+v", inner)
	}
	if ast.StartLine != 1 || ast.StartColumn != 1 || ast.EndColumn != 5 {
		t.Fatalf("root positions: %+v", ast)
	}
	innermost := inner.Children[0].(*ASTNode)
	if len(innermost.Children) != 1 || innermost.StartLine != 0 {
		t.Fatalf("innermost: %+v", innermost)
	}
}
