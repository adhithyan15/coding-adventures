package parser

// Tests for the left-recursion analysis (leftrec.go): what it marks, and the
// proof that skipping the growth loop for unmarked rules changes nothing
// observable. The proof is a differential: every grammar is compiled twice,
// once normally and once with compileGrammar's forceLeftRec knob (growth loop
// after every rule, the old behaviour), and the two must agree on the tree,
// the error text, and the engine's accounting.

import (
	"fmt"
	"math/rand"
	"sort"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// leftRecNames compiles src and returns the sorted names of the rules marked
// possibly-left-recursive.
func leftRecNames(t *testing.T, src string) []string {
	t.Helper()
	c, err := CompileGrammar(mustGrammar(t, strings.ReplaceAll(src, " ; ", " ;\n")))
	if err != nil {
		t.Fatal(err)
	}
	return markedNames(c)
}

func markedNames(c *Compiled) []string {
	var out []string
	for _, r := range c.rules {
		if r.leftRec {
			out = append(out, r.name)
		}
	}
	sort.Strings(out)
	return out
}

func TestLeftRecAnalysis(t *testing.T) {
	for _, tc := range []struct {
		name, src string
		want      []string
	}{
		{"control: no recursion", `a = b X ; b = Y ;`, nil},
		{"control: right recursion only", `l = LP l RP | NUM ;`, nil},
		{"control: recursion after a token", `a = X a | Y ;`, nil},
		{"control: recursion inside a repetition after a token", `a = X { a } ;`, nil},
		{"direct", `e = e PLUS t | t ; t = NUM ;`, []string{"e"}},
		{"indirect", `a = b X | Y ; b = a Z | W ;`, []string{"a", "b"}},
		{"indirect through a third rule", `a = b ; b = c ; c = a X | Y ;`, []string{"a", "b", "c"}},
		{"through a nullable prefix", `a = [ X ] a Y | Z ;`, []string{"a"}},
		{"through a nullable rule", `a = n a Y | Z ; n = { X } ;`, []string{"a"}},
		{"through a rule made nullable by a fixpoint", `a = n2 a Y | Z ; n2 = n1 ; n1 = [ X ] ;`, []string{"a"}},
		{"through a positive lookahead", `a = &a X | Y ;`, []string{"a"}},
		{"through a negative lookahead", `a = !a X | Y ;`, []string{"a"}},
		{"through a lookahead used as a nullable prefix", `a = !X a Y | Z ;`, []string{"a"}},
		{"through a positive lookahead prefix of another rule", `a = &Z b ; b = a Y | Z ;`, []string{"a", "b"}},
		{"through an optional lookahead", `a = [ &X ] a Y | Z ;`, []string{"a"}},
		{"through a lookahead then more", `a = &b X ; b = a Y ;`, []string{"a", "b"}},
		{"through EOF", `a = EOF a | X ;`, []string{"a"}},
		{"through a separated repetition element", `a = { a // COMMA } ;`, []string{"a"}},
		{"through a separated repetition separator", `a = { X // a } ;`, []string{"a"}},
		{"through a zero-or-more separated repetition prefix", `a = { X // COMMA } a Y | Z ;`, []string{"a"}},
		{"through an at-least-one separated repetition with a nullable element", `a = { [ X ] // COMMA }+ a Y | Z ;`, []string{"a"}},
		{"control: at-least-one separated repetition of a token", `a = { X // COMMA }+ a Y | Z ;`, nil},
		{"through a one-or-more", `a = { a }+ ;`, []string{"a"}},
		{"through a group and an alternation", `a = ( X | ( a Y ) ) ;`, []string{"a"}},
		{"only the cycle is marked", `s = a ; a = a X | Y ; z = Y ;`, []string{"a"}},
		{"undefined rule is not a cycle", `a = missing a2 | X ; a2 = Y ;`, nil},
		{"non-nullable prefix stops the walk", `a = X b ; b = a Y | Z ;`, nil},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if got := leftRecNames(t, tc.src); fmt.Sprint(got) != fmt.Sprint(tc.want) {
				t.Fatalf("marked %v, want %v", got, tc.want)
			}
		})
	}
}

// An empty literal can match the zero-width EOF, so it is nullable and does
// not hide a left call behind it.
func TestLeftRecAnalysisEmptyLiteral(t *testing.T) {
	ref := func(n string, tok bool) grammartools.GrammarElement {
		return grammartools.RuleReference{Name: n, IsToken: tok}
	}
	g := func(lit string) *grammartools.ParserGrammar {
		return &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
			{Name: "a", Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{
				grammartools.Literal{Value: lit}, ref("a", false), ref("X", true)}}},
		}}
	}
	for lit, want := range map[string]bool{"": true, "x": false} {
		c, err := CompileGrammar(g(lit))
		if err != nil {
			t.Fatal(err)
		}
		if c.rules[0].leftRec != want {
			t.Fatalf("literal %q: leftRec = %v, want %v", lit, c.rules[0].leftRec, want)
		}
	}
}

// A token reference of the EOF token type (by name) is nullable too, however
// it is spelled.
func TestLeftRecAnalysisEOFSpellings(t *testing.T) {
	c, err := CompileGrammar(&grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
		{Name: "a", Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{
			grammartools.RuleReference{Name: "EOF", IsToken: true},
			grammartools.RuleReference{Name: "a"}}}},
	}})
	if err != nil {
		t.Fatal(err)
	}
	if !c.rules[0].leftRec {
		t.Fatal("EOF-prefixed self reference must be marked")
	}
}

func TestForceLeftRecMarksEverything(t *testing.T) {
	c, err := compileGrammar(mustGrammar(t, "a = X b ;\nb = Y ;"), true)
	if err != nil {
		t.Fatal(err)
	}
	if got := markedNames(c); fmt.Sprint(got) != "[a b]" {
		t.Fatalf("forced marking = %v", got)
	}
}

// ---------------------------------------------------------------------------
// The differential: analysis on versus forced off.
// ---------------------------------------------------------------------------

// statParse parses with the compiled grammar and returns a canonical string
// of everything observable: the tree (rule names, positions, nil-ness of
// children) or the error text, plus the engine's internal accounting.
func statParse(c *Compiled, toks []lexer.Token, opts Options) string {
	if len(c.rules) == 0 {
		return "no rules"
	}
	s := c.pool.Get().(*parseState)
	s.reset(toks, opts)
	ast, err := s.run(0)
	var sb strings.Builder
	if err != nil {
		sb.WriteString("ERR " + err.Error())
	} else {
		dumpNew(ast, &sb)
	}
	fmt.Fprintf(&sb, " | slots=%d depth=%d frames=%d exceeded=%v pos=%d furthest=%d exp=%v",
		len(s.slots), s.depth, s.frames, s.exceeded, s.pos, s.furthestPos, s.furthestExpected)
	s.release()
	return sb.String()
}

var leftRecOptionSets = []Options{
	{},
	{MaxDepth: 5},
	{MaxDepth: 12},
	{MaxMemoSlots: 9},
	{MaxMemoSlots: 40},
	{MaxDepth: 8, MaxMemoSlots: 60},
}

func checkAnalysisEquivalent(t testing.TB, label string, pg *grammartools.ParserGrammar, rng *rand.Rand, inputs int) {
	on, err := compileGrammar(pg, false)
	if err != nil {
		t.Fatal(err)
	}
	off, err := compileGrammar(pg, true)
	if err != nil {
		t.Fatal(err)
	}
	for it := 0; it < inputs; it++ {
		toks := diffTokens(rng, rng.Intn(14))
		for _, opts := range leftRecOptionSets {
			a, b := statParse(on, toks, opts), statParse(off, toks, opts)
			if a != b {
				var names []string
				for _, tk := range toks {
					names = append(names, tk.TypeName)
				}
				t.Fatalf("%s: analysis changed the result (opts %+v) on %v\n analysis %s\n forced   %s",
					label, opts, names, a, b)
			}
		}
	}
}

func TestLeftRecAnalysisDifferentialFixedGrammars(t *testing.T) {
	n := 400
	if testing.Short() {
		n = 60
	}
	rng := rand.New(rand.NewSource(11))
	for _, g := range diffGrammars {
		pg := mustGrammar(t, strings.ReplaceAll(g.src, " ; ", " ;\n"))
		checkAnalysisEquivalent(t, g.name, pg, rng, n)
	}
	// The benchmark-shaped grammars too, and grammars the frozen old engine
	// cannot run (it recurses forever on a rule that matches the zero-width
	// EOF repeatedly), which is why they are not in diffGrammars.
	for name, src := range map[string]string{"json": jsonLikeGrammar, "expr": exprGrammar, "bracket": bracketGrammar,
		"eof-left":    "p = a ;\na = EOF a | a Y | Z ;",
		"eof-left2":   "a = EOF a Y | Z | NUM ;",
		"sep-prefix":  "a = { X // COMMA } a Y | Z | NUM ;",
		"sep-prefix2": "a = { [ X ] // PLUS }+ a Y | Z | NUM ;"} {
		checkAnalysisEquivalent(t, name, mustGrammar(t, src), rng, n/4)
	}
}

// randomGrammar builds a small random grammar of nrules rules over the diff
// vocabulary. It deliberately mixes the shapes the analysis reasons about:
// lookaheads, optional and repeated parts, separated repetitions, the EOF
// token, the empty literal and an undefined rule name.
func randomGrammar(rng *rand.Rand, nrules int) *grammartools.ParserGrammar {
	ruleName := func(i int) string { return fmt.Sprintf("r%d", i) }
	var gen func(depth int) grammartools.GrammarElement
	gen = func(depth int) grammartools.GrammarElement {
		kinds := 14
		if depth <= 0 {
			kinds = 5 // leaves only
		}
		switch rng.Intn(kinds) {
		case 0, 1:
			return grammartools.RuleReference{Name: ruleName(rng.Intn(nrules))}
		case 2:
			return grammartools.RuleReference{Name: diffVocab[rng.Intn(len(diffVocab)-1)], IsToken: true}
		case 3:
			if rng.Intn(4) == 0 {
				return grammartools.RuleReference{Name: "EOF", IsToken: true}
			}
			return grammartools.RuleReference{Name: ruleName(rng.Intn(nrules))}
		case 4:
			switch rng.Intn(5) {
			case 0:
				return grammartools.Literal{Value: ""}
			case 1:
				return grammartools.RuleReference{Name: "undefined_rule"}
			}
			v := diffVocab[rng.Intn(len(diffVocab)-1)]
			return grammartools.Literal{Value: diffVals[v]}
		case 5, 6:
			n := 1 + rng.Intn(3)
			els := make([]grammartools.GrammarElement, n)
			for i := range els {
				els[i] = gen(depth - 1)
			}
			return grammartools.Sequence{Elements: els}
		case 7, 8:
			n := 2 + rng.Intn(2)
			els := make([]grammartools.GrammarElement, n)
			for i := range els {
				els[i] = gen(depth - 1)
			}
			return grammartools.Alternation{Choices: els}
		case 9:
			return grammartools.Optional{Element: gen(depth - 1)}
		case 10:
			return grammartools.Repetition{Element: gen(depth - 1)}
		case 11:
			if rng.Intn(2) == 0 {
				return grammartools.PositiveLookahead{Element: gen(depth - 1)}
			}
			return grammartools.NegativeLookahead{Element: gen(depth - 1)}
		case 12:
			if rng.Intn(2) == 0 {
				return grammartools.Group{Element: gen(depth - 1)}
			}
			return grammartools.OneOrMoreRepetition{Element: gen(depth - 1)}
		default:
			return grammartools.SeparatedRepetition{Element: gen(depth - 1),
				Separator: gen(depth - 1), AtLeastOne: rng.Intn(2) == 0}
		}
	}
	pg := &grammartools.ParserGrammar{}
	for i := 0; i < nrules; i++ {
		pg.Rules = append(pg.Rules, grammartools.GrammarRule{Name: ruleName(i), Body: gen(3)})
	}
	return pg
}

func TestLeftRecAnalysisDifferentialRandomGrammars(t *testing.T) {
	grammars, inputs := 500, 12
	if testing.Short() {
		grammars, inputs = 80, 6
	}
	rng := rand.New(rand.NewSource(2024))
	marked, total := 0, 0
	for gi := 0; gi < grammars; gi++ {
		pg := randomGrammar(rng, 2+rng.Intn(4))
		checkAnalysisEquivalent(t, fmt.Sprintf("random grammar %d", gi), pg, rng, inputs)
		c, _ := compileGrammar(pg, false)
		for _, r := range c.rules {
			total++
			if r.leftRec {
				marked++
			}
		}
	}
	// The generator must exercise both sides of the analysis, or the
	// differential proves nothing.
	if marked == 0 || marked == total {
		t.Fatalf("random grammars marked %d of %d rules; need a mix", marked, total)
	}
}

// Skipping the loop must still produce the left-associative growth for the
// rules that do need it, and the expected-set text must still dedupe by value.
func TestExpectedSetDedupesByText(t *testing.T) {
	// Two different elements print the same text: a token reference named
	// X and a rule named X-shaped text cannot collide in the grammar syntax,
	// but a rule and a token can share a name. Both must report once.
	c, err := CompileGrammar(&grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
		{Name: "s", Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{
			grammartools.RuleReference{Name: "NUM", IsToken: true},
			grammartools.RuleReference{Name: "NUM", IsToken: true},
			grammartools.Literal{Value: "z"},
			grammartools.Literal{Value: "z"},
		}}},
	}})
	if err != nil {
		t.Fatal(err)
	}
	_, err = c.Parse([]lexer.Token{{TypeName: "Y", Value: "y", Line: 1, Column: 1}, {TypeName: "EOF", Line: 1, Column: 2}}, Options{})
	if err == nil || err.Error() == "" {
		t.Fatal("expected a parse error")
	}
	if !strings.Contains(err.Error(), `Expected NUM or "z" or s, got "y"`) {
		t.Fatalf("unexpected message %q", err.Error())
	}
}
