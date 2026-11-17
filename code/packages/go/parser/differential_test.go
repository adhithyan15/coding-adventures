package parser

// Differential tests: the new compiled engine against a frozen copy of the
// old map-memo engine (internal/oldengine), over adversarial grammars and
// random token streams. A "canonical dump" of each result (rule names, node
// positions, nil-ness of Children, token types and columns) or of the error
// text is compared as a string.
//
// Only inputs the old engine terminates on are generated (every stream ends
// with EOF; no grammar has a zero-width repetition body), because the old
// engine loops or overflows the stack otherwise. Those cases, where the new
// engine deliberately differs, are covered by explicit tests in
// compiled_test.go and are the whole of the allow-list:
//
//   - token list without EOF (old: unbounded recursion on `(`, spin on a
//     trailing NEWLINE),
//   - zero-width repetition body (old: infinite loop),
//   - nil grammar / empty token list (old: panic recovered as an error),
//   - nil vs empty Children on a memo hit (old: nil for a repetition-only
//     rule, empty otherwise; new: always empty). allowedNilDivergence
//     accepts old-nil/new-empty and nothing else.

import (
	"fmt"
	"math/rand"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	old "github.com/adhithyan15/coding-adventures/code/packages/go/parser/internal/oldengine"
)

var diffGrammars = []struct{ name, src string }{
	{"direct-left", `e = e PLUS t | e MINUS t | t ; t = t STAR f | f ; f = NUM | LP e RP ;`},
	{"indirect-left", `a = b X | Y ; b = a Z | W ;`},
	{"indirect-left2", `s = a ; a = b X | c ; b = a Y | a Z ; c = NUM | LP a RP ;`},
	{"nested-left", `e = e PLUS e | e STAR e | NUM | LP e RP ;`},
	{"left-in-rep", `p = { e SEMI } ; e = e PLUS NUM | NUM | n ; n = e X ;`},
	{"lookahead", `p = { i } ; i = &NUM NUM [ X ] | !X !NUM Y | X &( Y | Z ) | o ; o = [ Z ] W ;`},
	{"nullable-rules", `p = a b c ; a = [ X ] ; b = { Y } ; c = a NUM a | a W ;`},
	{"nullable-memo", `p = x q | x r ; x = [ X ] ; q = Y Z ; r = Y W | NUM ;`},
	{"look-memo", `p = x q | x r ; x = &Y | !Z ; q = Y Z ; r = Y W | NUM ;`},
	{"seprep", `p = { NUM // X } Y { l // SEMI }+ ; l = LP { NUM // PLUS } RP | NUM ;`},
	{"newline-insig", `p = { s } ; s = NUM [ "x" ] SEMI | Y [ Z ] | LP { s } RP ;`},
	{"newline-sig", `p = { s NEWLINE } ; s = NUM [ "x" ] | Y { Z } | LP s RP ;`},
	{"literals", `p = { "x" | "y" "z" | n } ; n = NUM "+" n | NUM ;`},
	{"oneplus", `p = { a }+ ; a = { X }+ Y | { NUM }+ [ Z ] | LP p RP ;`},
	{"undefined", `p = { a } ; a = X missing | X Y | NUM ;`},
	{"deepchain", `p = e ; e = a { PLUS a } ; a = m { STAR m } ; m = u ; u = MINUS u | q ; q = NUM | LP e RP | q2 ; q2 = X [ e ] Y ;`},
}

var diffVocab = []string{"NUM", "PLUS", "MINUS", "STAR", "LP", "RP", "X", "Y", "Z", "W", "SEMI", "NEWLINE"}
var diffVals = map[string]string{"NUM": "1", "PLUS": "+", "MINUS": "-", "STAR": "*", "LP": "(", "RP": ")",
	"X": "x", "Y": "y", "Z": "z", "W": "w", "SEMI": ";", "NEWLINE": "\n"}

func dumpOld(n *old.ASTNode, sb *strings.Builder) {
	fmt.Fprintf(sb, "(%s@%d:%d-%d:%d nil=%v", n.RuleName, n.StartLine, n.StartColumn, n.EndLine, n.EndColumn, n.Children == nil)
	for _, c := range n.Children {
		switch x := c.(type) {
		case *old.ASTNode:
			dumpOld(x, sb)
		case lexer.Token:
			fmt.Fprintf(sb, " %s:%d", x.TypeName, x.Column)
		}
	}
	sb.WriteString(")")
}

func dumpNew(n *ASTNode, sb *strings.Builder) {
	fmt.Fprintf(sb, "(%s@%d:%d-%d:%d nil=%v", n.RuleName, n.StartLine, n.StartColumn, n.EndLine, n.EndColumn, n.Children == nil)
	for _, c := range n.Children {
		switch x := c.(type) {
		case *ASTNode:
			dumpNew(x, sb)
		case lexer.Token:
			fmt.Fprintf(sb, " %s:%d", x.TypeName, x.Column)
		}
	}
	sb.WriteString(")")
}

// allowedNilDivergence: the only accepted difference. The old engine's memo
// replays raw children, which is nil for a rule whose body is a bare
// repetition; the new engine always yields an empty non-nil slice. Never the
// other way round.
func allowedNilDivergence(oldDump, newDump string) bool {
	return oldDump != newDump &&
		strings.ReplaceAll(oldDump, "nil=true", "nil=false") == newDump &&
		!strings.Contains(newDump, "nil=true")
}

func diffTokens(rng *rand.Rand, n int) []lexer.Token {
	toks := make([]lexer.Token, 0, n+1)
	for i := 0; i < n; i++ {
		v := diffVocab[rng.Intn(len(diffVocab))]
		if v == "NEWLINE" && rng.Intn(3) != 0 {
			v = "NUM"
		}
		toks = append(toks, lexer.Token{TypeName: v, Value: diffVals[v], Line: 1, Column: i + 1})
	}
	return append(toks, lexer.Token{TypeName: "EOF", Line: 1, Column: n + 1})
}

// compareOne parses toks with both engines and reports a mismatch ("" if none).
func compareOne(name string, old1 func() (string, bool), new1 func() (string, bool)) string {
	od, _ := old1()
	nd, _ := new1()
	if od != nd && !allowedNilDivergence(od, nd) {
		return fmt.Sprintf("%s:\n old %s\n new %s", name, od, nd)
	}
	return ""
}

func diffGrammarByName(t testing.TB) map[string]*Compiled {
	out := map[string]*Compiled{}
	for _, g := range diffGrammars {
		c, err := CompileGrammar(mustGrammar(t, strings.ReplaceAll(g.src, " ; ", " ;\n")))
		if err != nil {
			t.Fatal(err)
		}
		out[g.name] = c
	}
	return out
}

func runDiff(t testing.TB, idx int, toks []lexer.Token, compiled map[string]*Compiled) {
	g := diffGrammars[idx%len(diffGrammars)]
	pg := mustGrammar(t, strings.ReplaceAll(g.src, " ; ", " ;\n"))
	oa, oe := old.NewGrammarParser(append([]lexer.Token(nil), toks...), pg).Parse()
	na, ne := compiled[g.name].Parse(toks, Options{})
	odump := func() (string, bool) {
		if oe != nil {
			return "ERR " + oe.Error(), false
		}
		var sb strings.Builder
		dumpOld(oa, &sb)
		return sb.String(), true
	}
	ndump := func() (string, bool) {
		if ne != nil {
			return "ERR " + ne.Error(), false
		}
		var sb strings.Builder
		dumpNew(na, &sb)
		return sb.String(), true
	}
	if msg := compareOne(g.name, odump, ndump); msg != "" {
		var names []string
		for _, tk := range toks {
			names = append(names, tk.TypeName)
		}
		t.Fatalf("differential mismatch on %v\n%s", names, msg)
	}
}

func TestDifferentialAgainstFrozenOldEngine(t *testing.T) {
	n := 3000
	if testing.Short() {
		n = 300
	}
	rng := rand.New(rand.NewSource(7))
	compiled := diffGrammarByName(t)
	for gi := range diffGrammars {
		for it := 0; it < n; it++ {
			runDiff(t, gi, diffTokens(rng, rng.Intn(14)), compiled)
		}
	}
}

// FuzzDifferential lets `go test -fuzz=FuzzDifferential` hunt for streams
// where the engines disagree. Bytes pick tokens from the vocabulary.
func FuzzDifferential(f *testing.F) {
	f.Add(byte(0), []byte{0, 1, 0, 2, 0})
	f.Add(byte(3), []byte{4, 0, 1, 0, 5})
	f.Add(byte(11), []byte{0, 11, 6, 11})
	compiled := diffGrammarByName(f)
	f.Fuzz(func(t *testing.T, gsel byte, data []byte) {
		if len(data) > 24 {
			data = data[:24]
		}
		toks := make([]lexer.Token, 0, len(data)+1)
		for i, b := range data {
			v := diffVocab[int(b)%len(diffVocab)]
			toks = append(toks, lexer.Token{TypeName: v, Value: diffVals[v], Line: 1, Column: i + 1})
		}
		toks = append(toks, lexer.Token{TypeName: "EOF", Line: 1, Column: len(data) + 1})
		runDiff(t, int(gsel), toks, compiled)
	})
}
