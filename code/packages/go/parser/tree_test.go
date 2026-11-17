package parser

// Tests for the compact API (tree.go): ParseInto, Tree and its accessors,
// ToAST. The contract under test is that nothing observable about a parse
// changes when it is taken through ParseInto: same success or failure, same
// error text and cause, the same tree once ToAST has rebuilt it (positions,
// nil-ness, node and slice sharing), and the accessors report that same
// structure. The big three-way differential against the frozen engine lives
// in differential_test.go (runDiff) and the identity golden in
// identity_golden_test.go; this file holds the direct tests.

import (
	"errors"
	"fmt"
	"math/rand"
	"strings"
	"sync"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// dumpTree writes the subtree at n using ONLY the public accessors, in
// dumpNew's format so the two walks are comparable as strings. Recursive:
// for small trees only.
func dumpTree(t *Tree, n NodeID, sb *strings.Builder) {
	sl, sc, el, ec := t.Position(n)
	fmt.Fprintf(sb, "(%s@%d:%d-%d:%d nil=false", t.Rule(n), sl, sc, el, ec)
	for i := 0; i < t.NumChildren(n); i++ {
		ch := t.Child(n, i)
		if ch.IsToken() {
			tok := t.Token(ch.Token())
			fmt.Fprintf(sb, " %s:%d", tok.TypeName, tok.Column)
		} else {
			dumpTree(t, ch.Node(), sb)
		}
	}
	sb.WriteString(")")
}

// compareParse parses with Parse and ParseInto (into tr) and requires the
// same outcome. deep skips the recursive dumps (for trees tens of thousands of
// levels deep) and compares only the outcome.
func compareParse(t *testing.T, label string, c *Compiled, tr *Tree, toks []lexer.Token, opts Options, deep bool) (ok bool) {
	t.Helper()
	ast, perr := c.Parse(toks, opts)
	ierr := c.ParseInto(tr, toks, opts)
	if (perr == nil) != (ierr == nil) {
		t.Fatalf("%s: Parse err %v, ParseInto err %v", label, perr, ierr)
	}
	if perr != nil {
		if perr.Error() != ierr.Error() {
			t.Fatalf("%s: error text differs: %q vs %q", label, perr, ierr)
		}
		for _, lim := range []error{ErrMaxDepth, ErrMaxTokens, ErrMaxMemoSlots, ErrUnknownStartRule} {
			if errors.Is(perr, lim) != errors.Is(ierr, lim) {
				t.Fatalf("%s: cause differs for %v", label, lim)
			}
		}
		var pe, ie *GrammarParseError
		if errors.As(perr, &pe) != errors.As(ierr, &ie) || (pe != nil && pe.Tok != ie.Tok) {
			t.Fatalf("%s: error token differs", label)
		}
		if tr.Root() != NoNode || tr.ToAST() != nil || tr.NumNodes() != 0 || tr.NumTokens() != 0 {
			t.Fatalf("%s: tree not empty after error %v", label, ierr)
		}
		return false
	}
	if tr.Root() == NoNode {
		t.Fatalf("%s: no root after success", label)
	}
	if deep {
		return true
	}
	var a, b, w strings.Builder
	dumpNew(ast, &a)
	dumpNew(tr.ToAST(), &b)
	dumpTree(tr, tr.Root(), &w)
	if a.String() != b.String() || a.String() != w.String() {
		t.Fatalf("%s:\n parse %s\n toast %s\n walk  %s", label, a.String(), b.String(), w.String())
	}
	return true
}

const pbracketGrammar = `v = LB [ v ] RB | NUM ;`

func bracketToks(n int) []lexer.Token {
	toks := bracketsAround([]lexer.Token{benchTok("NUM", "1")}, n)
	return append(toks, benchTok("EOF", ""))
}

// The limits, swept so that every outcome (accept, refuse at each limit) is
// compared between Parse and ParseInto.
func TestParseIntoLimitsMatchParse(t *testing.T) {
	var tr Tree
	c := mustCompile(t, pbracketGrammar)
	for n := 0; n < 20; n++ {
		toks := bracketToks(n)
		for d := 1; d < 24; d++ {
			compareParse(t, fmt.Sprintf("MaxDepth n=%d d=%d", n, d), c, &tr, toks, Options{MaxDepth: d}, false)
		}
		for m := 1; m < 70; m++ {
			compareParse(t, fmt.Sprintf("MaxMemoSlots n=%d m=%d", n, m), c, &tr, toks, Options{MaxMemoSlots: m}, false)
		}
		for m := 1; m < 45; m++ {
			compareParse(t, fmt.Sprintf("MaxTokens n=%d m=%d", n, m), c, &tr, toks, Options{MaxTokens: m}, false)
		}
	}
	// A refusal really is a refusal, and not vacuous: each limit fires.
	for _, tc := range []struct {
		opts Options
		want error
	}{{Options{MaxDepth: 3}, ErrMaxDepth}, {Options{MaxMemoSlots: 3}, ErrMaxMemoSlots}, {Options{MaxTokens: 3}, ErrMaxTokens}} {
		if err := c.ParseInto(&tr, bracketToks(10), tc.opts); !errors.Is(err, tc.want) {
			t.Fatalf("want %v, got %v", tc.want, err)
		}
	}

	// Left recursion: the tree grows deeper than the call stack.
	lc := mustCompile(t, leftRecGrammar)
	for _, n := range []int{1, 5, 30} {
		for d := 1; d < 40; d += 3 {
			compareParse(t, fmt.Sprintf("leftrec n=%d d=%d", n, d), lc, &tr, leftChainTokens(n), Options{MaxDepth: d}, false)
		}
	}
	if err := lc.ParseInto(&tr, leftChainTokens(200_000), Options{}); !errors.Is(err, ErrMaxDepth) || tr.Root() != NoNode {
		t.Fatalf("200k-term chain: %v", err)
	}

	// Syntax errors, an unknown start rule, an empty token list.
	compareParse(t, "syntax", c, &tr, []lexer.Token{benchTok("LB", "["), benchTok("EOF", "")}, Options{}, false)
	compareParse(t, "unknown start", c, &tr, bracketToks(1), Options{StartRule: "nope"}, false)
	compareParse(t, "no tokens", c, &tr, nil, Options{}, false)
	compareParse(t, "start rule", c, &tr, bracketToks(2), Options{StartRule: "v"}, false)
}

// The frame budget (HardMaxFrames) and the height tests, through ParseInto.
func TestParseIntoFrameBudgetAndHeight(t *testing.T) {
	var tr Tree
	const groups = 50
	body := strings.Repeat("( ", groups) + "LBRACKET [ a ] RBRACKET" + strings.Repeat(" )", groups)
	c := mustCompile(t, "a = "+body+" | NUMBER ;")
	if compareParse(t, "frames", c, &tr, nestedTokens(1_000_000), Options{MaxDepth: int(^uint(0) >> 1)}, true) {
		t.Fatal("the frame budget should have refused")
	}
	if err := c.ParseInto(&tr, nestedTokens(1_000_000), Options{MaxDepth: int(^uint(0) >> 1)}); !strings.Contains(err.Error(), "stack-frame budget") {
		t.Fatalf("frame budget message: %v", err)
	}
	compareParse(t, "shallow", c, &tr, nestedTokens(100), Options{}, true)

	// A memo hit replayed deeper than it was built (height_test.go).
	const k = 8
	var chain strings.Builder
	for i := 1; i < k; i++ {
		fmt.Fprintf(&chain, "c%d = c%d ;\n", i, i+1)
	}
	hc := mustCompile(t, "v = t1 X | c1 ;\nt1 = LB v RB ;\n"+chain.String()+fmt.Sprintf("c%d = LB v RB | NUM ;", k))
	toks := append(bracketsAround([]lexer.Token{benchTok("NUM", "1")}, 4900), benchTok("EOF", ""))
	if compareParse(t, "deeper-hit", hc, &tr, toks, Options{}, true) {
		t.Fatal("expected a depth refusal")
	}
	small := append(bracketsAround([]lexer.Token{benchTok("NUM", "1")}, 20), benchTok("EOF", ""))
	compareParse(t, "deeper-hit-small", hc, &tr, small, Options{}, false)

	// A long left chain within limits builds a tree 100k levels deep: ToAST
	// must not recurse (it would hit the Go stack otherwise).
	lc := mustCompile(t, leftRecGrammar)
	chainToks := leftChainTokens(30_000)
	if !compareParse(t, "long chain", lc, &tr, chainToks, Options{MaxDepth: 50_000}, true) {
		t.Fatal("long chain refused")
	}
	if ast := tr.ToAST(); ast == nil || ast.StartLine != 1 {
		t.Fatal("ToAST of a deep tree")
	}
}

// The flat index limit (int32 index space) fails closed through ParseInto
// exactly as through Parse, at every limit, leaving the Tree empty.
func TestParseIntoFlatOverflow(t *testing.T) {
	defer func(l int) { flatIndexLimit = l }(flatIndexLimit)
	var tr Tree
	for gi, src := range []string{
		`p = &x x y ; x = A B ; y = { z } ; z = C ;`,
		`p = &x &x &x &x &x &x &x &x x ; x = A ;`,
		`e = e PLUS NUM | NUM ;`,
	} {
		c := mustCompile(t, src)
		toks := []lexer.Token{tokAt("A", "a", 1, 1), tokAt("B", "b", 1, 2)}
		for i := 0; i < 8; i++ {
			toks = append(toks, tokAt("C", "c", 1, 3+i))
		}
		if gi == 1 {
			toks = []lexer.Token{tokAt("A", "a", 1, 1)}
		}
		if gi == 2 {
			toks = []lexer.Token{tokAt("NUM", "1", 1, 1)}
			for i := 0; i < 8; i++ {
				toks = append(toks, tokAt("PLUS", "+", 1, 2+2*i), tokAt("NUM", "1", 1, 3+2*i))
			}
		}
		toks = append(toks, tokAt("EOF", "", 1, 99))
		sawErr, sawOK := false, false
		for limit := 0; limit < 90; limit++ {
			flatIndexLimit = limit
			if compareParse(t, fmt.Sprintf("g%d limit %d", gi, limit), c, &tr, toks, Options{}, false) {
				sawOK = true
			} else {
				sawErr = true
			}
		}
		if !sawErr || !sawOK {
			t.Fatalf("grammar %d: sweep did not hit both outcomes", gi)
		}
	}
}

// Random grammars, tokens and limits (the identity-golden generators), with
// ONE Tree reused throughout, so reuse across success and every kind of error
// is exercised. The 200,000-parse hashed version is TestIdentityGoldenParseInto.
func TestParseIntoRandomGrammars(t *testing.T) {
	rng := rand.New(rand.NewSource(99))
	var tr Tree
	n := 3000
	if testing.Short() {
		n = 300
	}
	for gi := 0; gi < n; gi++ {
		nr := 1 + rng.Intn(6)
		c, err := CompileGrammar(idGrammar(rng, nr, rng.Intn(3) == 0))
		if err != nil {
			t.Fatal(err)
		}
		for it := 0; it < 5; it++ {
			opts := Options{}
			switch rng.Intn(5) {
			case 0:
				opts.MaxDepth = 1 + rng.Intn(14)
			case 1:
				opts.MaxMemoSlots = 1 + rng.Intn(60)
			case 2:
				opts.MaxTokens = 1 + rng.Intn(8)
			}
			if rng.Intn(2) == 0 {
				opts.StartRule = fmt.Sprintf("r%d", rng.Intn(nr))
			}
			compareParse(t, fmt.Sprintf("g%d/%d", gi, it), c, &tr, idTokens(rng), opts, false)
		}
	}
}

// A memo hit is a distinct node, reached through a child slot, that shares the
// children of the node it replays and has no Position; Span still reports the
// real token range. `p = &x x y` parses x, discards the lookahead, and the
// second x is replayed from the memo.
func TestTreeMemoHitAccessors(t *testing.T) {
	c := mustCompile(t, `p = &x x y ; x = A B ; y = C ;`)
	toks := []lexer.Token{tokAt("A", "a", 1, 1), tokAt("B", "b", 1, 2), tokAt("C", "c", 2, 7), tokAt("EOF", "", 2, 8)}
	var tr Tree
	if err := c.ParseInto(&tr, toks, Options{}); err != nil {
		t.Fatal(err)
	}
	root := tr.Root()
	if tr.Rule(root) != "p" || tr.NumChildren(root) != 2 || tr.IsReplay(root) {
		t.Fatalf("root %s children=%d", tr.Rule(root), tr.NumChildren(root))
	}
	x := tr.Child(root, 0)
	if x.IsToken() || x.Token() != -1 {
		t.Fatal("child 0 should be a node")
	}
	xn := x.Node()
	if !tr.IsReplay(xn) || tr.Rule(xn) != "x" || tr.NumChildren(xn) != 2 {
		t.Fatalf("expected a replay of x, got %s replay=%v", tr.Rule(xn), tr.IsReplay(xn))
	}
	if sl, sc, el, ec := tr.Position(xn); sl|sc|el|ec != 0 {
		t.Fatalf("a replay has zero Position, got %d:%d-%d:%d", sl, sc, el, ec)
	}
	if f, l := tr.Span(xn); f != 0 || l != 1 {
		t.Fatalf("replay Span = %d,%d", f, l)
	}
	if sl, sc, el, ec := tr.Position(root); sl != 1 || sc != 1 || el != 2 || ec != 7 {
		t.Fatalf("root Position %d:%d-%d:%d", sl, sc, el, ec)
	}
	y := tr.Child(root, 1).Node()
	if tr.IsReplay(y) || tr.Rule(y) != "y" {
		t.Fatal("y is a plain node")
	}
	if ch := tr.Child(xn, 0); !ch.IsToken() || ch.Node() != NoNode || tr.Token(ch.Token()).Value != "a" {
		t.Fatal("x's first child is token a")
	}
	// The same shape through ToAST: distinct node, shared Children.
	ast := tr.ToAST()
	hit := ast.Children[0].(*ASTNode)
	if hit.StartLine != 0 || len(hit.Children) != 2 {
		t.Fatalf("ToAST replay: %+v", hit)
	}
	id, ok := c.RuleID("x")
	if !ok || tr.RuleID(xn) != id || c.RuleName(id) != "x" {
		t.Fatal("RuleID/RuleName")
	}
	if _, ok := c.RuleID("nope"); ok {
		t.Fatal("unknown rule has no id")
	}
	if c.RuleName(-1) != "" || c.RuleName(1000) != "" {
		t.Fatal("RuleName out of range")
	}

	// The synthetic EOF is token index NumTokens.
	c2 := mustCompile(t, `p = A EOF ;`)
	if err := c2.ParseInto(&tr, []lexer.Token{tokAt("A", "a", 3, 4)}, Options{}); err != nil {
		t.Fatal(err)
	}
	last := tr.Child(tr.Root(), 1)
	if !last.IsToken() || last.Token() != tr.NumTokens() || tr.Token(last.Token()).TypeName != "EOF" {
		t.Fatal("EOF child")
	}
	if _, _, el, ec := tr.Position(tr.Root()); el != 3 || ec != 4 {
		t.Fatalf("EOF carries the last token's position, got %d:%d", el, ec)
	}

	// A node with no tokens has no Span and zero Position.
	c3 := mustCompile(t, `p = e A ; e = [ B ] ;`)
	if err := c3.ParseInto(&tr, []lexer.Token{tokAt("A", "a", 1, 1)}, Options{}); err != nil {
		t.Fatal(err)
	}
	e := tr.Child(tr.Root(), 0).Node()
	if f, l := tr.Span(e); f != -1 || l != -1 {
		t.Fatalf("empty node Span %d,%d", f, l)
	}
	if sl, _, _, _ := tr.Position(e); sl != 0 || tr.NumChildren(e) != 0 {
		t.Fatal("empty node")
	}
}

// Misuse panics cleanly rather than reading outside the tree.
func TestTreeAccessorPanics(t *testing.T) {
	c := mustCompile(t, `p = A ;`)
	var tr Tree
	if err := c.ParseInto(&tr, []lexer.Token{tokAt("A", "a", 1, 1)}, Options{}); err != nil {
		t.Fatal(err)
	}
	for name, f := range map[string]func(){
		"child high": func() { tr.Child(tr.Root(), 1) },
		"child low":  func() { tr.Child(tr.Root(), -1) },
		"token high": func() { tr.Token(tr.NumTokens() + 1) },
	} {
		func() {
			defer func() {
				if recover() == nil {
					t.Fatalf("%s: no panic", name)
				}
			}()
			f()
		}()
	}
}

// Empty trees, the zero Tree, nil Tree, PostParse refusal, hooks.
func TestParseIntoEdges(t *testing.T) {
	c := mustCompile(t, `p = A ;`)
	toks := []lexer.Token{tokAt("A", "a", 1, 1)}
	var zero Tree
	if zero.Root() != NoNode || zero.ToAST() != nil || zero.NumNodes() != 0 {
		t.Fatal("zero Tree must be empty")
	}
	if err := c.ParseInto(nil, toks, Options{}); err == nil {
		t.Fatal("nil Tree must be an error")
	}
	var tr Tree
	if err := c.ParseInto(&tr, toks, Options{}); err != nil {
		t.Fatal(err)
	}
	// PostParse is refused, and the previous contents are discarded first.
	err := c.ParseInto(&tr, toks, Options{PostParse: []PostParseHook{func(a *ASTNode) *ASTNode { return a }}})
	if err == nil || tr.Root() != NoNode {
		t.Fatalf("PostParse: err=%v root=%v", err, tr.Root())
	}
	// PreParse runs, and the Tree refers to the transformed tokens.
	err = c.ParseInto(&tr, nil, Options{PreParse: []PreParseHook{func([]lexer.Token) []lexer.Token { return toks }}})
	if err != nil || tr.NumTokens() != 1 {
		t.Fatalf("PreParse: %v", err)
	}
	// A panicking hook is an error and leaves the Tree empty.
	err = c.ParseInto(&tr, toks, Options{PreParse: []PreParseHook{func([]lexer.Token) []lexer.Token { panic("boom") }}})
	if err == nil || !strings.Contains(err.Error(), "internal error") || tr.Root() != NoNode {
		t.Fatalf("panic: err=%v", err)
	}
	// ... and the Compiled still works afterwards.
	if err := c.ParseInto(&tr, toks, Options{}); err != nil || tr.Rule(tr.Root()) != "p" {
		t.Fatalf("after panic: %v", err)
	}
	// An empty grammar.
	empty := &Compiled{}
	if err := empty.ParseInto(&tr, toks, Options{}); err == nil || tr.Root() != NoNode {
		t.Fatal("empty grammar")
	}
	tr.Reset()
	if tr.Root() != NoNode || tr.NumTokens() != 0 {
		t.Fatal("Reset")
	}
}

// Parse big, then small, then an error, then small, into one Tree. Each result
// is right; the Tree is empty after the error; and the retained capacity is
// capped (a huge result is held only until the next use).
func TestTreeReuseAndCaps(t *testing.T) {
	var tr Tree
	wide := mustCompile(t, `p = { NUM } ;`)
	n := 3 * maxPooledFlat
	wideToks := make([]lexer.Token, 0, n+1)
	for i := 0; i < n; i++ {
		wideToks = append(wideToks, tokAt("NUM", "1", 1, i+1))
	}
	wideToks = append(wideToks, tokAt("EOF", "", 1, n+1))
	if err := wide.ParseInto(&tr, wideToks, Options{}); err != nil {
		t.Fatal(err)
	}
	if cap(tr.kids) <= maxPooledFlat || tr.NumChildren(tr.Root()) != n {
		t.Fatalf("expected a big retained result, cap(kids)=%d", cap(tr.kids))
	}
	if sl, _, el, ec := tr.Position(tr.Root()); sl != 1 || el != 1 || ec != n {
		t.Fatalf("big Position %d %d %d", sl, el, ec)
	}
	if got := tr.ToAST(); len(got.Children) != n {
		t.Fatalf("big ToAST: %d", len(got.Children))
	}
	// Small: the oversized buffers are dropped, not reused.
	small := []lexer.Token{tokAt("NUM", "1", 1, 1), tokAt("EOF", "", 1, 2)}
	if err := wide.ParseInto(&tr, small, Options{}); err != nil {
		t.Fatal(err)
	}
	if cap(tr.kids) > maxPooledFlat || cap(tr.nodes) > maxPooledFlat || cap(tr.first) > maxPooledFlat {
		t.Fatalf("oversized buffer retained: kids=%d nodes=%d", cap(tr.kids), cap(tr.nodes))
	}
	tr.Span(tr.Root())
	if tr.NumChildren(tr.Root()) != 1 {
		t.Fatal("small result wrong")
	}
	// Error: empty, and still bounded.
	if err := wide.ParseInto(&tr, []lexer.Token{tokAt("Y", "y", 1, 1)}, Options{}); err == nil {
		t.Fatal("expected a syntax error")
	}
	if tr.Root() != NoNode || tr.ToAST() != nil || len(tr.kids) != 0 || len(tr.nodes) != 0 || tr.tokens != nil {
		t.Fatal("tree not empty after the error")
	}
	// Small again: correct, and equal to Parse.
	var cmp Tree
	compareParse(t, "after error", wide, &tr, small, Options{}, false)
	compareParse(t, "fresh tree", wide, &cmp, small, Options{}, false)

	// An oversized span cache is also dropped by Reset.
	tr.first = make([]int32, 0, maxPooledFlat+1)
	tr.last = make([]int32, 0, maxPooledFlat+1)
	tr.Reset()
	if tr.first != nil || tr.last != nil {
		t.Fatal("Reset kept an oversized span cache")
	}
}

// Steady state: one Tree, reused, allocates nothing per parse. (The memo
// table and scratch stack come from the Compiled's pool, the arrays are the
// Tree's own.)
func TestParseIntoSteadyStateAllocatesNothing(t *testing.T) {
	if raceEnabled {
		t.Skip("sync.Pool drops items at random under -race")
	}
	c := mustCompile(t, exprGrammar)
	toks := exprTokens(20)
	var tr Tree
	if err := c.ParseInto(&tr, toks, Options{}); err != nil {
		t.Fatal(err)
	}
	allocs := testing.AllocsPerRun(50, func() {
		if err := c.ParseInto(&tr, toks, Options{}); err != nil {
			t.Fatal(err)
		}
	})
	if allocs != 0 {
		t.Fatalf("ParseInto allocated %v per parse in steady state", allocs)
	}
}

// A Tree owns its arrays: parsing something else with the same Compiled (and
// so the same pooled states) never changes an existing Tree.
func TestTreeDoesNotAliasPooledState(t *testing.T) {
	c := mustCompile(t, exprGrammar)
	var a, b Tree
	if err := c.ParseInto(&a, exprTokens(3), Options{}); err != nil {
		t.Fatal(err)
	}
	var before strings.Builder
	dumpTree(&a, a.Root(), &before)
	for i := 0; i < 20; i++ {
		if err := c.ParseInto(&b, exprTokens(1+i), Options{}); err != nil {
			t.Fatal(err)
		}
		if _, err := c.Parse(exprTokens(2+i), Options{}); err != nil {
			t.Fatal(err)
		}
		_ = c.ParseInto(&b, []lexer.Token{benchTok("PLUS", "+")}, Options{})
	}
	var after strings.Builder
	dumpTree(&a, a.Root(), &after)
	if before.String() != after.String() {
		t.Fatal("a Tree changed under later parses")
	}
}

// Distinct Trees on one Compiled, in parallel (run with -race).
func TestParseIntoConcurrent(t *testing.T) {
	c := mustCompile(t, exprGrammar)
	want := map[int]string{}
	for i := 1; i <= 5; i++ {
		ast, err := c.Parse(exprTokens(i), Options{})
		if err != nil {
			t.Fatal(err)
		}
		want[i] = dumpString(ast)
	}
	var wg sync.WaitGroup
	errs := make(chan string, 16)
	for g := 0; g < 8; g++ {
		wg.Add(1)
		go func(g int) {
			defer wg.Done()
			var tr Tree
			for it := 0; it < 200; it++ {
				k := 1 + (g+it)%5
				if it%7 == 3 {
					if c.ParseInto(&tr, []lexer.Token{benchTok("PLUS", "+")}, Options{}) == nil {
						errs <- "expected an error"
						return
					}
					continue
				}
				if err := c.ParseInto(&tr, exprTokens(k), Options{}); err != nil {
					errs <- err.Error()
					return
				}
				if got := dumpString(tr.ToAST()); got != want[k] {
					errs <- "wrong tree"
					return
				}
				tr.Span(tr.Root())
			}
		}(g)
	}
	wg.Wait()
	close(errs)
	for e := range errs {
		t.Fatal(e)
	}
}
