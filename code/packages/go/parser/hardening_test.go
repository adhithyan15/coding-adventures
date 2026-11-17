package parser

// Regression tests from the G2 review: left recursion and element nesting
// must not be able to overflow the stack, hook panics must not escape,
// resource limits, nil-vs-empty children, pool reuse, and the compiled-grammar
// cache.

import (
	"errors"
	"fmt"
	"reflect"
	"runtime/debug"
	"strings"
	"sync"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// limitStack lowers Go's goroutine stack cap for one test so that an engine
// that recursed per input element would crash (fatal, uncatchable) instead of
// quietly using a few hundred MB.
func limitStack(t *testing.T, bytes int) {
	t.Helper()
	old := debug.SetMaxStack(bytes)
	t.Cleanup(func() { debug.SetMaxStack(old) })
}

// leftChainTokens is `1 + 1 + 1 ... + 1` with n terms.
func leftChainTokens(n int) []lexer.Token {
	toks := make([]lexer.Token, 0, 2*n)
	for i := 0; i < n; i++ {
		if i > 0 {
			toks = append(toks, benchTok("PLUS", "+"))
		}
		toks = append(toks, benchTok("NUMBER", "1"))
	}
	return append(toks, benchTok("EOF", ""))
}

const leftRecGrammar = `e = e PLUS NUMBER | NUMBER ;`

// The security MEDIUM: seed growing is a loop, so call depth stays at 1 while
// the TREE grows one level per term, and computeNodePosition recursed down
// the spine. 7M terms used to end in "fatal error: stack overflow". With an
// 8 MB stack cap, 1M terms would have been enough to crash the old code.
func TestLeftRecursionIsBoundedByMaxDepth(t *testing.T) {
	limitStack(t, 8<<20)
	c, err := CompileGrammar(mustGrammar(t, leftRecGrammar))
	if err != nil {
		t.Fatal(err)
	}
	ast, err := c.Parse(leftChainTokens(1_000_000), Options{})
	requireDepthError(t, err)
	if ast != nil {
		t.Fatal("no tree with a depth error")
	}
	// Several left-recursive alternatives share the same growth loop.
	c2, _ := CompileGrammar(mustGrammar(t, `e = e PLUS NUMBER | e MINUS NUMBER | NUMBER ;`))
	toks := []lexer.Token{benchTok("NUMBER", "1")}
	for i := 0; i < 200_000; i++ {
		toks = append(toks, benchTok("MINUS", "-"), benchTok("NUMBER", "1"))
	}
	toks = append(toks, benchTok("EOF", ""))
	_, err = c2.Parse(toks, Options{})
	requireDepthError(t, err)
}

// Within the limit a long left-associative chain still parses, with correct
// positions on every node, and its tree can be walked.
func TestLongLeftChainWithinLimit(t *testing.T) {
	limitStack(t, 8<<20)
	c, _ := CompileGrammar(mustGrammar(t, leftRecGrammar))
	const n = 4000
	toks := leftChainTokens(n)
	for i := range toks {
		toks[i].Line, toks[i].Column = 1, i+1
	}
	ast, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	if ast.StartColumn != 1 || ast.EndColumn != 2*n-1 {
		t.Fatalf("root span = %d..%d", ast.StartColumn, ast.EndColumn)
	}
	if got := len(FindNodes(ast, "e")); got != n {
		t.Fatalf("FindNodes found %d e-nodes, want %d", got, n)
	}
	if got := len(CollectTokens(ast, "NUMBER")); got != n {
		t.Fatalf("CollectTokens found %d numbers, want %d", got, n)
	}
}

// The tree helpers must be safe on a caller-built tree of any depth: the
// recursive versions overflowed at roughly 3.5M nodes (WalkAST) with the
// default 1 GB cap, and far sooner under this 8 MB one.
func TestTreeHelpersAreIterative(t *testing.T) {
	limitStack(t, 8<<20)
	const depth = 1_000_000
	leaf := &ASTNode{RuleName: "n", Children: []interface{}{benchTok("NUMBER", "1")}}
	root := leaf
	for i := 0; i < depth; i++ {
		root = &ASTNode{RuleName: "n", Children: []interface{}{root, benchTok("PLUS", "+")}}
	}
	if got := len(FindNodes(root, "n")); got != depth+1 {
		t.Fatalf("FindNodes = %d", got)
	}
	if got := len(CollectTokens(root, "PLUS")); got != depth {
		t.Fatalf("CollectTokens = %d", got)
	}
	if findFirstToken(root.Children) == nil || findLastToken(root.Children) == nil {
		t.Fatal("first/last token not found")
	}
	computeNodePosition(root) // also iterative, and O(1) per level
	// A rewriting walk, with Leave replacing every leaf-level node, rebuilds
	// the whole spine without recursion.
	out := WalkAST(root, ASTVisitor{Leave: func(n *ASTNode, _ *ASTNode) *ASTNode {
		if len(n.Children) == 1 {
			return &ASTNode{RuleName: "leaf", Children: n.Children}
		}
		return nil
	}})
	if out == root || len(FindNodes(out, "leaf")) != 1 {
		t.Fatal("rewrite did not take effect")
	}
}

// walkNode's iterative rewrite must visit in the same order a recursive walk
// would: Enter pre-order, Leave post-order, with the right parent.
func TestWalkOrderAndParents(t *testing.T) {
	mk := func(name string, kids ...interface{}) *ASTNode { return &ASTNode{RuleName: name, Children: kids} }
	tree := mk("a", mk("b", mk("d")), benchTok("X", "x"), mk("c"))
	var log []string
	WalkAST(tree, ASTVisitor{
		Enter: func(n, p *ASTNode) *ASTNode {
			pn := "-"
			if p != nil {
				pn = p.RuleName
			}
			log = append(log, "E"+n.RuleName+"<"+pn)
			return nil
		},
		Leave: func(n, p *ASTNode) *ASTNode {
			log = append(log, "L"+n.RuleName)
			return nil
		},
	})
	want := "Ea<- Eb<a Ed<b Ld Lb Ec<a Lc La"
	if got := strings.Join(log, " "); got != want {
		t.Fatalf("order %q, want %q", got, want)
	}
}

// Fix 4: only parseRule was counted, so a grammar whose RULE body is deeply
// nested (groups, optionals) used `nest` Go frames per rule level that no
// limit saw. The frame budget now charges that nesting, so even a huge
// MaxDepth cannot overflow a (here deliberately small) stack.
func TestNestedElementsAreChargedToFrameBudget(t *testing.T) {
	const groups = 50
	body := strings.Repeat("( ", groups) + "LBRACKET [ a ] RBRACKET" + strings.Repeat(" )", groups)
	c, err := CompileGrammar(mustGrammar(t, "a = "+body+" | NUMBER ;"))
	if err != nil {
		t.Fatal(err)
	}
	limitStack(t, 128<<20)
	// A max-depth request (clamped to HardMaxDepth = 50,000) would need
	// 50,000 x 50 frames without the budget.
	_, err = c.Parse(nestedTokens(1_000_000), Options{MaxDepth: int(^uint(0) >> 1)})
	requireDepthError(t, err)
	// And ordinary nesting well inside the budget still parses.
	if _, err := c.Parse(nestedTokens(100), Options{}); err != nil {
		t.Fatalf("shallow nesting failed: %v", err)
	}
}

func TestRuleWeights(t *testing.T) {
	c, _ := CompileGrammar(mustGrammar(t, `
flat = NUMBER ;
nested = ( ( [ NUMBER ] ) ) ;
sep = { NUMBER // COMMA } ;
`))
	w := map[string]int{}
	for _, r := range c.rules {
		w[r.name] = r.weight
	}
	if !(w["flat"] < w["nested"] && w["sep"] > w["flat"]) {
		t.Fatalf("weights should grow with nesting: %v", w)
	}
}

// Review fix 2: panics in wrapper hooks must not escape Parse.
func TestWrapperHookPanicsAreErrors(t *testing.T) {
	g := mustGrammar(t, `program = NUMBER ;`)
	toks := []lexer.Token{benchTok("NUMBER", "1"), benchTok("EOF", "")}

	p := NewGrammarParser(toks, g)
	p.AddPreParse(func([]lexer.Token) []lexer.Token { panic("pre boom") })
	if ast, err := p.Parse(); err == nil || ast != nil || !strings.Contains(err.Error(), "pre boom") {
		t.Fatalf("pre hook panic: %v %v", ast, err)
	}

	p = NewGrammarParser(toks, g)
	p.AddPostParse(func(*ASTNode) *ASTNode { panic("post boom") })
	if ast, err := p.Parse(); err == nil || ast != nil || !strings.Contains(err.Error(), "post boom") {
		t.Fatalf("post hook panic: %v %v", ast, err)
	}
}

// Fix 5: a memo hit for a rule that matched nothing must have the same
// non-nil empty Children as the first parse (JSON `[]`, not `null`).
func TestNilVersusEmptyChildren(t *testing.T) {
	c, _ := CompileGrammar(mustGrammar(t, `
p = x y | x z ;
x = { NAME } ;
y = [ NAME ] NUMBER ;
z = [ NAME ] [ COMMA ] ;
`))
	// `x` matches nothing at position 0, twice: once parsed, once from memo.
	ast, err := c.Parse([]lexer.Token{benchTok("EOF", "")}, Options{})
	if err != nil {
		t.Fatal(err)
	}
	var check func(n *ASTNode)
	check = func(n *ASTNode) {
		if n.Children == nil {
			t.Fatalf("rule %s has nil Children", n.RuleName)
		}
		for _, ch := range n.Children {
			if sub, ok := ch.(*ASTNode); ok {
				check(sub)
			}
		}
	}
	check(ast)
	// The first alternative's x was parsed (then abandoned); the surviving x
	// in the tree is the memo-hit copy, which is the case under test.
	if got := len(FindNodes(ast, "x")); got != 1 {
		t.Fatalf("expected the memo-hit x in the tree, got %d", got)
	}
}

func TestMaxTokensAndMemoSlots(t *testing.T) {
	c, _ := CompileGrammar(mustGrammar(t, exprGrammar))
	toks := exprTokens(20)

	_, err := c.Parse(toks, Options{MaxTokens: 10})
	if !errors.Is(err, ErrMaxTokens) {
		t.Fatalf("expected ErrMaxTokens, got %v", err)
	}
	if _, err := c.Parse(toks, Options{MaxTokens: len(toks)}); err != nil {
		t.Fatalf("exactly at the token limit must pass: %v", err)
	}

	_, err = c.Parse(toks, Options{MaxMemoSlots: 5})
	if !errors.Is(err, ErrMaxMemoSlots) {
		t.Fatalf("expected ErrMaxMemoSlots, got %v", err)
	}
	var pe *GrammarParseError
	if !errors.As(err, &pe) || !strings.Contains(pe.Message, "(5)") {
		t.Fatalf("message should name the limit: %v", err)
	}
	if _, err := c.Parse(toks, Options{MaxMemoSlots: 1 << 20}); err != nil {
		t.Fatalf("generous slot limit must pass: %v", err)
	}
}

// The pool must hand back clean state after a depth error, a memo-limit
// error and a panic.
func TestPoolReuseAfterFailures(t *testing.T) {
	c, _ := CompileGrammar(mustGrammar(t, bracketGrammar))
	good := nestedTokens(5)
	want, err := c.Parse(good, Options{})
	if err != nil {
		t.Fatal(err)
	}
	check := func(label string) {
		t.Helper()
		got, err := c.Parse(good, Options{})
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: pooled state leaked: %v", label, err)
		}
	}
	_, err = c.Parse(nestedTokens(500), Options{MaxDepth: 20})
	requireDepthError(t, err)
	check("after depth error")

	_, err = c.Parse(good, Options{MaxMemoSlots: 2})
	if !errors.Is(err, ErrMaxMemoSlots) {
		t.Fatal(err)
	}
	check("after memo-limit error")

	_, err = c.Parse(good, Options{PostParse: []PostParseHook{func(*ASTNode) *ASTNode { panic("x") }}})
	if err == nil {
		t.Fatal("expected panic error")
	}
	check("after hook panic")

	// A syntax error, too.
	if _, err := c.Parse([]lexer.Token{benchTok("RBRACKET", "]"), benchTok("EOF", "")}, Options{}); err == nil {
		t.Fatal("expected syntax error")
	}
	check("after syntax error")
}

// Many goroutines parse a mix of valid, invalid and too-deep inputs on one
// Compiled; every result must equal what a lone parse gives. Run with -race.
func TestConcurrentMixedParses(t *testing.T) {
	c, _ := CompileGrammar(mustGrammar(t, exprGrammar))
	type input struct {
		toks []lexer.Token
		opts Options
	}
	bad := append(exprTokens(3)[:5:5], benchTok("PLUS", "+"), benchTok("EOF", ""))
	inputs := []input{
		{exprTokens(2), Options{}},
		{exprTokens(30), Options{}},
		{bad, Options{}},
		{exprTokens(30), Options{MaxDepth: 5}},
		{exprTokens(4), Options{StartRule: "additive"}},
	}
	type outcome struct {
		ast *ASTNode
		msg string
	}
	run := func(in input) outcome {
		ast, err := c.Parse(in.toks, in.opts)
		if err != nil {
			return outcome{msg: err.Error()}
		}
		return outcome{ast: ast}
	}
	want := make([]outcome, len(inputs))
	for i, in := range inputs {
		want[i] = run(in)
	}
	var wg sync.WaitGroup
	for w := 0; w < 12; w++ {
		wg.Add(1)
		go func(w int) {
			defer wg.Done()
			for round := 0; round < 40; round++ {
				i := (round + w) % len(inputs)
				if got := run(inputs[i]); !reflect.DeepEqual(got, want[i]) {
					t.Errorf("goroutine %d input %d: result differs from sequential run", w, i)
					return
				}
			}
		}(w)
	}
	wg.Wait()
}

func TestCompiledGrammarCache(t *testing.T) {
	g := mustGrammar(t, exprGrammar)
	p1 := NewGrammarParser(exprTokens(1), g)
	p2 := NewGrammarParser(exprTokens(2), g)
	if p1.compiled != p2.compiled {
		t.Fatal("the same *ParserGrammar must compile once")
	}
	ForgetGrammar(g)
	p3 := NewGrammarParser(exprTokens(1), g)
	if p3.compiled == p1.compiled {
		t.Fatal("ForgetGrammar must drop the cached form")
	}
	ForgetGrammar(g)
	ForgetGrammar(g) // forgetting an absent grammar is a no-op
	ForgetGrammar(nil)

	// A mutated grammar is only seen after ForgetGrammar (documented rule).
	m := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
		{Name: "r", Body: grammartools.RuleReference{Name: "NAME", IsToken: true}},
	}}
	toks := []lexer.Token{benchTok("NUMBER", "1"), benchTok("EOF", "")}
	if _, err := NewGrammarParser(toks, m).Parse(); err == nil {
		t.Fatal("NAME grammar should reject a NUMBER")
	}
	m.Rules[0].Body = grammartools.RuleReference{Name: "NUMBER", IsToken: true}
	ForgetGrammar(m)
	if _, err := NewGrammarParser(toks, m).Parse(); err != nil {
		t.Fatalf("after ForgetGrammar the new rules apply: %v", err)
	}
}

// The cache is bounded: beyond maxCachedGrammars distinct grammars, parsers
// still work (compiled per call) and the cache stops growing.
func TestCompiledGrammarCacheIsBounded(t *testing.T) {
	var gs []*grammartools.ParserGrammar
	t.Cleanup(func() {
		for _, g := range gs {
			ForgetGrammar(g)
		}
	})
	toks := []lexer.Token{benchTok("NUMBER", "1"), benchTok("EOF", "")}
	for i := 0; i < maxCachedGrammars+20; i++ {
		g := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
			{Name: fmt.Sprintf("r%d", i), Body: grammartools.RuleReference{Name: "NUMBER", IsToken: true}},
		}}
		gs = append(gs, g)
		if _, err := NewGrammarParser(toks, g).Parse(); err != nil {
			t.Fatal(err)
		}
	}
	if n := compiledCacheSize.Load(); n > maxCachedGrammars {
		t.Fatalf("cache grew to %d", n)
	}
}

// Documented EOF behaviour: a synthetic EOF past the end satisfies a trailing
// EOF in the grammar; an EOF in the middle ends the parse.
func TestEOFSemantics(t *testing.T) {
	c, _ := CompileGrammar(mustGrammar(t, `p = A B EOF ;`))
	if _, err := c.Parse([]lexer.Token{benchTok("A", "a"), benchTok("B", "b")}, Options{}); err != nil {
		t.Fatalf("synthetic EOF should satisfy the grammar's EOF: %v", err)
	}
	c2, _ := CompileGrammar(mustGrammar(t, `p = A ;`))
	mid := []lexer.Token{benchTok("A", "a"), benchTok("EOF", ""), benchTok("B", "ignored"), benchTok("EOF", "")}
	if _, err := c2.Parse(mid, Options{}); err != nil {
		t.Fatalf("a mid-stream EOF ends the parse (documented): %v", err)
	}
}
