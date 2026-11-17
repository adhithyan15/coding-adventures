package parser

import (
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// The zero-width-EOF cases the review found. A rule that reaches itself at the
// synthetic EOF is left-recursive for the engine: the growth re-run cannot
// lengthen the match, but its memo hit on the seed runs the height check and
// can raise ErrMaxDepth at a tight MaxDepth. Both nullable spellings (EOF and
// the empty literal) must keep the loop, so analysis-on and forced-on agree.
func TestLeftRecAnalysisDifferentialZeroWidthEOF(t *testing.T) {
	eofTok := lexer.Token{TypeName: "EOF", Value: "", Line: 1, Column: 1}
	body := func(first grammartools.GrammarElement) grammartools.GrammarElement {
		return grammartools.Sequence{Elements: []grammartools.GrammarElement{first,
			grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{
				grammartools.Sequence{Elements: []grammartools.GrammarElement{
					grammartools.RuleReference{Name: "a"}, grammartools.RuleReference{Name: "NUM", IsToken: true}}},
				grammartools.RuleReference{Name: "Y", IsToken: true},
				grammartools.RuleReference{Name: "EOF", IsToken: true},
			}}}}}
	}
	for name, first := range map[string]grammartools.GrammarElement{
		"EOF token":     grammartools.RuleReference{Name: "EOF", IsToken: true},
		"empty literal": grammartools.Literal{Value: ""},
	} {
		pg := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{{Name: "a", Body: body(first)}}}
		on, _ := compileGrammar(pg, false)
		off, _ := compileGrammar(pg, true)
		if !on.rules[0].leftRec {
			t.Fatalf("%s: a rule reaching itself at the synthetic EOF must be marked", name)
		}
		for _, tc := range []struct {
			toks     []lexer.Token
			maxDepth int
		}{{nil, 1}, {[]lexer.Token{eofTok}, 2}, {nil, 2}, {[]lexer.Token{eofTok}, 1}, {nil, 0}} {
			opts := Options{MaxDepth: tc.maxDepth}
			if got, want := statParse(on, tc.toks, opts), statParse(off, tc.toks, opts); got != want {
				t.Fatalf("%s %v depth %d:\n on  %s\n off %s", name, tc.toks, tc.maxDepth, got, want)
			}
		}
		// The refusal itself is observable, so the test has teeth.
		if got := statParse(on, nil, Options{MaxDepth: 1}); !strings.Contains(got, "ERR") {
			t.Fatalf("%s: expected a depth refusal at MaxDepth 1, got %s", name, got)
		}
	}
}

// The review's seed-4242 counterexample: a start rule other than the first, an
// undefined rule, a left call through EOF, and a single EOF token.
func TestLeftRecAnalysisDifferentialSeed4242(t *testing.T) {
	ref := func(n string) grammartools.GrammarElement { return grammartools.RuleReference{Name: n} }
	tok := func(n string) grammartools.GrammarElement { return grammartools.RuleReference{Name: n, IsToken: true} }
	seq := func(e ...grammartools.GrammarElement) grammartools.GrammarElement {
		return grammartools.Sequence{Elements: e}
	}
	alt := func(e ...grammartools.GrammarElement) grammartools.GrammarElement {
		return grammartools.Alternation{Choices: e}
	}
	pg := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{
		{Name: "r0", Body: seq(tok("EOF"), grammartools.Repetition{Element: ref("nope")},
			grammartools.Group{Element: alt(
				seq(grammartools.Group{Element: seq(ref("r0"), tok("NUM"), ref("r4"))},
					grammartools.PositiveLookahead{Element: ref("nope")}, grammartools.PositiveLookahead{Element: tok("EOF")}),
				grammartools.Literal{Value: ""}, tok("Y"))})},
		{Name: "r1", Body: tok("Y")},
		{Name: "r2", Body: ref("r0")},
		{Name: "r3", Body: alt(grammartools.Literal{Value: "NUM"}, ref("r1"),
			grammartools.Group{Element: alt(tok("X"), ref("r2"), tok("NUM"))})},
		{Name: "r4", Body: ref("r4")},
	}}
	on, _ := compileGrammar(pg, false)
	off, _ := compileGrammar(pg, true)
	toks := []lexer.Token{{TypeName: "EOF", Value: "", Line: 1, Column: 1}}
	opts := Options{MaxDepth: 4, StartRule: "r3"}
	if a, b := statParse(on, toks, opts), statParse(off, toks, opts); a != b {
		t.Fatalf("on  %s\n off %s", a, b)
	}
}
