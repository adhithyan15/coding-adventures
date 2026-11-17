package parser

// Tests for the compiled engine: recursion limits, start rules, reuse, and
// the properties the engine promises (no recovery, fail closed).

import (
	"errors"
	"fmt"
	"runtime/debug"
	"strings"
	"sync"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// bracketGrammar is the smallest grammar that recurses on nesting:
//
//	value = LBRACKET [ value ] RBRACKET | NUMBER
//
// `[[[1]]]` descends value -> value -> value, one rule level per bracket.
const bracketGrammar = `
value = LBRACKET [ value ] RBRACKET | NUMBER ;
`

// nestedTokens returns depth opening brackets, a number, depth closing
// brackets, EOF.
func nestedTokens(depth int) []lexer.Token {
	toks := make([]lexer.Token, 0, 2*depth+2)
	for i := 0; i < depth; i++ {
		toks = append(toks, benchTok("LBRACKET", "["))
	}
	toks = append(toks, benchTok("NUMBER", "1"))
	for i := 0; i < depth; i++ {
		toks = append(toks, benchTok("RBRACKET", "]"))
	}
	return append(toks, benchTok("EOF", ""))
}

func requireDepthError(t *testing.T, err error) {
	t.Helper()
	if err == nil {
		t.Fatal("expected a depth error, got nil")
	}
	if !errors.Is(err, ErrMaxDepth) {
		t.Fatalf("expected ErrMaxDepth, got %v", err)
	}
	var pe *GrammarParseError
	if !errors.As(err, &pe) {
		t.Fatalf("expected *GrammarParseError, got %T", err)
	}
	if !strings.Contains(pe.Message, "supported limit") {
		t.Fatalf("unexpected message %q", pe.Message)
	}
}

// The headline regression: a million brackets used to kill the process with
// "fatal error: stack overflow". Existing callers (NewGrammarParser) are
// protected by the default limit with no code change.
func TestDeepNestingReturnsErrorInsteadOfCrashing(t *testing.T) {
	g := mustGrammar(t, bracketGrammar)
	toks := nestedTokens(1_000_000)
	_, err := NewGrammarParser(toks, g).Parse()
	requireDepthError(t, err)
}

// The attacker shape from the bug report: f( followed by many [ then the
// closers. Built from a grammar with a call rule so the recursion crosses
// several rules per level.
func TestHostileCallAndBracketInput(t *testing.T) {
	g := mustGrammar(t, `
expr = NAME LPAREN [ items ] RPAREN | items ;
items = LBRACKET [ items ] RBRACKET ;
`)
	const n = 200_000
	toks := []lexer.Token{benchTok("NAME", "f"), benchTok("LPAREN", "(")}
	for i := 0; i < n; i++ {
		toks = append(toks, benchTok("LBRACKET", "["))
	}
	for i := 0; i < n; i++ {
		toks = append(toks, benchTok("RBRACKET", "]"))
	}
	toks = append(toks, benchTok("RPAREN", ")"), benchTok("EOF", ""))
	_, err := NewGrammarParser(toks, g).Parse()
	requireDepthError(t, err)
}

// The refusal happens inside an optional, which backtracks past it and would
// succeed on a truncated prefix. The sticky flag must still turn that into
// an error: no partial result, ever.
func TestDepthErrorIsNotSwallowedByBacktracking(t *testing.T) {
	g := mustGrammar(t, `
program = { item } ;
item = LBRACKET [ item ] RBRACKET | NUMBER ;
`)
	c, err := CompileGrammar(g)
	if err != nil {
		t.Fatal(err)
	}
	ast, err := c.Parse(nestedTokens(50), Options{MaxDepth: 10})
	requireDepthError(t, err)
	if ast != nil {
		t.Fatal("a depth error must not come with a tree")
	}
	// Control: the same input under a sufficient limit parses.
	if _, err := c.Parse(nestedTokens(50), Options{MaxDepth: 1000}); err != nil {
		t.Fatalf("control parse failed: %v", err)
	}
}

func TestMaxDepthBoundary(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, bracketGrammar))
	if err != nil {
		t.Fatal(err)
	}
	// `[1]` needs 2 rule levels (value, value); `[[1]]` needs 3.
	if _, err := c.Parse(nestedTokens(1), Options{MaxDepth: 2}); err != nil {
		t.Fatalf("depth 2 should fit exactly: %v", err)
	}
	_, err = c.Parse(nestedTokens(2), Options{MaxDepth: 2})
	requireDepthError(t, err)
}

func TestLegitimateNestingUnderDefaultLimit(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, bracketGrammar))
	if err != nil {
		t.Fatal(err)
	}
	// 5,000 nested brackets is far beyond real data and well inside the
	// default limit of 10,000 rule levels.
	if _, err := c.Parse(nestedTokens(5000), Options{}); err != nil {
		t.Fatalf("5000-deep nesting should parse: %v", err)
	}
}

func TestMaxDepthOptionNormalisation(t *testing.T) {
	cases := []struct{ in, want int }{
		{0, DefaultMaxDepth},
		{-5, DefaultMaxDepth},
		{7, 7},
		{HardMaxDepth, HardMaxDepth},
		{HardMaxDepth + 1, HardMaxDepth},
		{int(^uint(0) >> 1), HardMaxDepth},
	}
	for _, tc := range cases {
		if got := (Options{MaxDepth: tc.in}).maxDepth(); got != tc.want {
			t.Errorf("maxDepth(%d) = %d, want %d", tc.in, got, tc.want)
		}
	}
}

// Proves the claim in DefaultMaxDepth's comment: even a "no limit" request
// (clamped to HardMaxDepth) hostile input is refused with an error long
// before the goroutine stack limit, which this test lowers to 128 MB (one
// eighth of the real 64-bit cap) to make the point.
func TestHardMaxDepthFitsInStack(t *testing.T) {
	old := debug.SetMaxStack(128 << 20)
	defer debug.SetMaxStack(old)
	c, err := CompileGrammar(mustGrammar(t, bracketGrammar))
	if err != nil {
		t.Fatal(err)
	}
	_, err = c.Parse(nestedTokens(1_000_000), Options{MaxDepth: int(^uint(0) >> 1)})
	requireDepthError(t, err)
}

func TestStartRule(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
program = { stmt } ;
stmt = NAME EQUALS expr ;
expr = NUMBER { PLUS NUMBER } ;
`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{
		benchTok("NUMBER", "1"), benchTok("PLUS", "+"), benchTok("NUMBER", "2"), benchTok("EOF", ""),
	}
	// The default start rule (program) cannot parse a bare expression...
	if _, err := c.Parse(toks, Options{}); err == nil {
		t.Fatal("program should reject a bare expression")
	}
	// ...but starting from expr can.
	ast, err := c.Parse(toks, Options{StartRule: "expr"})
	if err != nil {
		t.Fatal(err)
	}
	if ast.RuleName != "expr" {
		t.Fatalf("root rule = %q, want expr", ast.RuleName)
	}
	// An unknown rule is an error, not a silent fallback to the first rule.
	_, err = c.Parse(toks, Options{StartRule: "nope"})
	if !errors.Is(err, ErrUnknownStartRule) {
		t.Fatalf("expected ErrUnknownStartRule, got %v", err)
	}
	// The wrapper constructor honours it too.
	ast, err = NewGrammarParserWithOptions(toks, mustGrammar(t, `
program = { stmt } ;
stmt = NAME EQUALS expr ;
expr = NUMBER { PLUS NUMBER } ;
`), Options{StartRule: "expr"}).Parse()
	if err != nil || ast.RuleName != "expr" {
		t.Fatalf("wrapper StartRule failed: %v %v", ast, err)
	}
}

// One Compiled, many parses, many goroutines: results equal the one-shot
// constructor's, and the pooled memo storage never leaks between parses.
func TestCompiledReuseMatchesOneShot(t *testing.T) {
	g := mustGrammar(t, exprGrammar)
	c, err := CompileGrammar(g)
	if err != nil {
		t.Fatal(err)
	}
	inputs := [][]lexer.Token{exprTokens(1), exprTokens(3), exprTokens(40), exprTokens(2)}
	want := make([]*ASTNode, len(inputs))
	for i, in := range inputs {
		want[i], err = NewGrammarParser(in, g).Parse()
		if err != nil {
			t.Fatal(err)
		}
	}
	var wg sync.WaitGroup
	for w := 0; w < 8; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for round := 0; round < 50; round++ {
				for i, in := range inputs {
					got, err := c.Parse(in, Options{})
					if err != nil {
						t.Error(err)
						return
					}
					if fmt.Sprint(got) != fmt.Sprint(want[i]) && !sameShape(got, want[i]) {
						t.Error("reused parse differs from one-shot parse")
						return
					}
				}
			}
		}()
	}
	wg.Wait()
}

func sameShape(a, b *ASTNode) bool {
	if a.RuleName != b.RuleName || len(a.Children) != len(b.Children) {
		return false
	}
	for i := range a.Children {
		x, xok := a.Children[i].(*ASTNode)
		y, yok := b.Children[i].(*ASTNode)
		if xok != yok {
			return false
		}
		if xok {
			if !sameShape(x, y) {
				return false
			}
		} else if a.Children[i].(lexer.Token) != b.Children[i].(lexer.Token) {
			return false
		}
	}
	return true
}

func TestCompileGrammarErrorsAndEdges(t *testing.T) {
	if _, err := CompileGrammar(nil); err == nil {
		t.Fatal("nil grammar must be an error")
	}
	// The wrapper reports it from Parse instead of panicking.
	p := NewGrammarParser(nestedTokens(1), nil)
	if _, err := p.Parse(); err == nil {
		t.Fatal("expected error from nil-grammar parser")
	}
	if p.NewlinesSignificant() {
		t.Fatal("nil grammar has no significant newlines")
	}
	// An empty grammar compiles but cannot parse.
	c, err := CompileGrammar(&grammartools.ParserGrammar{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := c.Parse(nestedTokens(1), Options{}); err == nil || !strings.Contains(err.Error(), "no rules") {
		t.Fatalf("expected 'no rules' error, got %v", err)
	}
}

// A duplicate rule name: the last definition wins (as in the old map-based
// engine), for the start rule and for references.
func TestDuplicateRuleLastWins(t *testing.T) {
	g := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
		{Name: "r", Body: grammartools.RuleReference{Name: "NAME", IsToken: true}},
		{Name: "r", Body: grammartools.RuleReference{Name: "NUMBER", IsToken: true}},
	}}
	c, err := CompileGrammar(g)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := c.Parse([]lexer.Token{benchTok("NUMBER", "1"), benchTok("EOF", "")}, Options{}); err != nil {
		t.Fatalf("last definition should win: %v", err)
	}
}

// No error recovery: bad input yields an error and a nil tree, never a
// partial result, and the error points at the furthest token reached.
func TestNoErrorRecovery(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, jsonLikeGrammar))
	if err != nil {
		t.Fatal(err)
	}
	// `[ 1 , , 2 ]`: a recovering parser might skip the stray comma.
	toks := []lexer.Token{
		benchTok("LBRACKET", "["), benchTok("NUMBER", "1"), benchTok("COMMA", ","),
		benchTok("COMMA", ","), benchTok("NUMBER", "2"), benchTok("RBRACKET", "]"), benchTok("EOF", ""),
	}
	ast, err := c.Parse(toks, Options{})
	if err == nil || ast != nil {
		t.Fatalf("expected (nil, error), got (%v, %v)", ast, err)
	}
	var pe *GrammarParseError
	if !errors.As(err, &pe) || errors.Is(err, ErrMaxDepth) {
		t.Fatalf("expected plain syntax *GrammarParseError, got %v", err)
	}
}

// A token list with no EOF must not make the parser read "past the end" as a
// repeat of the last token (which used to recurse forever on `(`).
func TestMissingEOFDoesNotRecurseForever(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, exprGrammar))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := c.Parse([]lexer.Token{benchTok("LPAREN", "(")}, Options{}); err == nil {
		t.Fatal("unterminated paren must be a syntax error")
	}
	// A well-formed list without EOF still parses.
	if _, err := c.Parse([]lexer.Token{benchTok("NUMBER", "1")}, Options{}); err != nil {
		t.Fatalf("1 without EOF should parse: %v", err)
	}
	// An empty token list is an error, not a panic.
	if _, err := c.Parse(nil, Options{}); err == nil {
		t.Fatal("empty input must be an error")
	}
}

// A trailing NEWLINE with no EOF used to spin forever in the newline skipper.
func TestTrailingNewlineWithoutEOF(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `file = { NAME } ;`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{benchTok("NAME", "a"), benchTok("NEWLINE", "\n")}
	if _, err := c.Parse(toks, Options{}); err != nil {
		t.Fatalf("expected success: %v", err)
	}
}

// A repetition whose body can match nothing must terminate.
func TestEmptyRepetitionTerminates(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
a = { [ NAME ] } ;
b = { NUMBER // [ COMMA ] }+ ;
c = { [ NAME ] }+ ;
`))
	if err != nil {
		t.Fatal(err)
	}
	eof := []lexer.Token{benchTok("EOF", "")}
	for _, rule := range []string{"a", "b", "c"} {
		toks := append([]lexer.Token{benchTok("NAME", "x"), benchTok("NUMBER", "1")}, eof...)
		_, _ = c.Parse(toks, Options{StartRule: rule}) // must return, either way
	}
}

func TestHooksViaOptionsAndPanicRecovery(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `program = NUMBER ;`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{benchTok("NAME", "x"), benchTok("EOF", "")}
	retype := func(in []lexer.Token) []lexer.Token {
		out := append([]lexer.Token(nil), in...)
		out[0].TypeName = "NUMBER"
		return out
	}
	var sawRule string
	ast, err := c.Parse(toks, Options{
		PreParse:  []PreParseHook{retype},
		PostParse: []PostParseHook{func(n *ASTNode) *ASTNode { sawRule = n.RuleName; return n }},
	})
	if err != nil || sawRule != "program" || ast == nil {
		t.Fatalf("hooks not applied: %v %q", err, sawRule)
	}
	// A panicking hook becomes an error rather than crashing the caller.
	_, err = c.Parse(toks, Options{PreParse: []PreParseHook{func([]lexer.Token) []lexer.Token { panic("boom") }}})
	if err == nil || !strings.Contains(err.Error(), "boom") {
		t.Fatalf("expected recovered panic error, got %v", err)
	}
}

func TestGrammarParserHooksAndTrace(t *testing.T) {
	g := mustGrammar(t, `program = NUMBER ;`)
	toks := []lexer.Token{benchTok("NAME", "x"), benchTok("EOF", "")}
	p := NewGrammarParserWithTrace(toks, g, true)
	p.AddPreParse(func(in []lexer.Token) []lexer.Token {
		out := append([]lexer.Token(nil), in...)
		out[0].TypeName = "NUMBER"
		return out
	})
	seen := false
	p.AddPostParse(func(n *ASTNode) *ASTNode { seen = true; return n })
	if _, err := p.Parse(); err != nil || !seen {
		t.Fatalf("hooks/trace parse failed: %v seen=%v", err, seen)
	}
	// Trace of a failing rule exercises the fail line.
	if _, err := NewGrammarParserWithTrace(toks, g, true).Parse(); err == nil {
		t.Fatal("expected failure")
	}
}

// Left recursion must still grow its seed correctly with the slice memo.
func TestLeftRecursionStillGrows(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, `
e = e PLUS NUMBER | NUMBER ;
`))
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{
		benchTok("NUMBER", "1"), benchTok("PLUS", "+"), benchTok("NUMBER", "2"),
		benchTok("PLUS", "+"), benchTok("NUMBER", "3"), benchTok("EOF", ""),
	}
	ast, err := c.Parse(toks, Options{})
	if err != nil {
		t.Fatal(err)
	}
	// Left-associative: ((1+2)+3) -> the root's first child is an e node.
	if len(CollectTokens(ast, "NUMBER")) != 3 {
		t.Fatalf("expected 3 numbers, got %v", ast)
	}
	if _, ok := ast.Children[0].(*ASTNode); !ok {
		t.Fatalf("root should wrap a nested e (left-assoc), got %T", ast.Children[0])
	}
}

// Covers every element kind through the compiled engine, including the
// predicates and the unknown-element and undefined-rule paths.
func TestAllElementKinds(t *testing.T) {
	g := mustGrammar(t, `
file = { item } ;
item = &NAME NAME | !NAME NUMBER | { COMMA }+ | { NUMBER // PLUS } ;
`)
	c, err := CompileGrammar(g)
	if err != nil {
		t.Fatal(err)
	}
	toks := []lexer.Token{benchTok("NAME", "a"), benchTok("NUMBER", "1"), benchTok("EOF", "")}
	if _, err := c.Parse(toks, Options{}); err != nil {
		t.Fatal(err)
	}
	// Undefined rule references fail instead of panicking.
	g2 := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
		{Name: "a", Body: grammartools.RuleReference{Name: "missing"}},
	}}
	c2, _ := CompileGrammar(g2)
	if _, err := c2.Parse([]lexer.Token{benchTok("EOF", "")}, Options{}); err == nil {
		t.Fatal("undefined rule must fail")
	}
	// An element type the engine does not know never matches.
	g3 := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{{Name: "a", Body: nil}}}
	c3, _ := CompileGrammar(g3)
	if _, err := c3.Parse([]lexer.Token{benchTok("EOF", "")}, Options{}); err == nil {
		t.Fatal("nil element must fail")
	}
}

func TestMemoPoolDropsHugeState(t *testing.T) {
	c, err := CompileGrammar(mustGrammar(t, bracketGrammar))
	if err != nil {
		t.Fatal(err)
	}
	// Force release()'s "too big to keep" branches without needing millions
	// of slots: shrink the notion via direct state manipulation.
	s := c.pool.Get().(*parseState)
	s.reset(nestedTokens(2), 10, false)
	s.slots = make([]memoSlot, 0, maxPooledSlots+1)
	s.head = make([]int32, maxPooledSlots+1)
	s.release()
	s2 := c.pool.Get().(*parseState)
	if cap(s2.slots) > maxPooledSlots {
		t.Fatal("oversized slots were pooled")
	}
	c.pool.Put(s2)
}

func TestGrammarParseErrorUnwrap(t *testing.T) {
	e := &GrammarParseError{Message: "x"}
	if e.Unwrap() != nil {
		t.Fatal("plain error has no cause")
	}
}

func BenchmarkParseCompiledJSONLike(b *testing.B) {
	benchmarkCompiled(b, jsonLikeGrammar, jsonLikeTokens(200))
}

func BenchmarkParseCompiledExpression(b *testing.B) {
	benchmarkCompiled(b, exprGrammar, exprTokens(200))
}

// BenchmarkParseCompiledSmallLine is the per-line use case the Kiln strace
// parser needs: one compiled grammar, thousands of tiny parses.
func BenchmarkParseCompiledSmallLine(b *testing.B) {
	benchmarkCompiled(b, exprGrammar, exprTokens(2))
}

func benchmarkCompiled(b *testing.B, src string, toks []lexer.Token) {
	c, err := CompileGrammar(mustGrammar(b, src))
	if err != nil {
		b.Fatal(err)
	}
	b.SetBytes(benchBytes(toks))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if _, err := c.Parse(toks, Options{}); err != nil {
			b.Fatal(err)
		}
	}
}
