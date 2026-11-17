package lexer

// All-grammar differential test and the dual-run switch
// =====================================================
//
// compiled_diff_test.go checks a hand-picked set of grammars. This file checks
// EVERY .tokens file under code/grammars that Go's regexp can compile (about
// 277 of them, including the F10-transition grammars) against the frozen legacy
// oracle, on generic samples, samples built from the grammar's own literals and
// keywords, and random mutations of both.
//
// Cost control (the full run is about 70 s):
//
//   - default:                every grammar, 4 mutations per sample
//   - -short:                 every 8th grammar, 2 mutations per sample
//   - LEXER_FULL_DIFF=1:      every grammar, 25 mutations per sample
//
// A CI job that wants the exhaustive run sets LEXER_FULL_DIFF=1.
//
// TestMain also turns on dualRunMatchers for the whole package (except when
// benchmarks are requested), so every lexer test, not only the differential
// ones, cross-checks each deterministic matcher against the regexp engine and
// panics on any disagreement.

import (
	"flag"
	"math/rand"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

func TestMain(m *testing.M) {
	flag.Parse()
	if f := flag.Lookup("test.bench"); f == nil || f.Value.String() == "" {
		dualRunMatchers = true
	}
	os.Exit(m.Run())
}

func allDefinitions(g *grammartools.TokenGrammar) []grammartools.TokenDefinition {
	out := append([]grammartools.TokenDefinition{}, g.Definitions...)
	out = append(out, g.SkipDefinitions...)
	for _, grp := range g.Groups {
		out = append(out, grp.Definitions...)
	}
	return out
}

func TestAllGrammarsDifferential(t *testing.T) {
	files, _ := filepath.Glob("../../../grammars/*/*.tokens")
	more, _ := filepath.Glob("../../../grammars/*/*/*.tokens")
	files = append(files, more...)
	if len(files) == 0 {
		t.Skip("no grammars found")
	}

	mutations, stride := 4, 1
	switch {
	case os.Getenv("LEXER_FULL_DIFF") != "":
		mutations = 25
	case testing.Short():
		mutations, stride = 2, 8
	}

	var generic []string
	for _, gc := range diffCases {
		generic = append(generic, gc.samples...)
	}
	generic = append(generic, straceLikeLines...)
	generic = append(generic,
		"fn main() { let s = r#\"a\"#; let c = 'a'; let l: &'a str = \"x\\n\"; /* c */ // d\n 1.0e3 0xff b'x' }\n",
		"`a ${b + `c${d}`} e` /re/g x = 1 / 2; <div a=\"b\">t</div>\n",
		"é\xff\"unterminated", "'''abc''' \"\"\"x\"\"\"")

	grammars, comparisons := 0, 0
	var nullableSkipped []string
	for fi, f := range files {
		if fi%stride != 0 {
			continue
		}
		data, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		g, err := grammartools.ParseTokenGrammar(string(data))
		if err != nil {
			continue
		}
		c, err := Compile(g)
		if err != nil {
			continue // Go's regexp cannot express it; the legacy engine failed too
		}
		// A rule that can match the empty string makes the legacy oracle loop
		// forever (Compile reports an error instead), so such grammars cannot
		// be compared.
		nullable := false
		var lits []string
		for _, d := range allDefinitions(g) {
			if d.IsRegex {
				if re, err := regexp.Compile("^" + d.Pattern); err == nil && re.MatchString("") {
					nullable = true
				}
			} else {
				if d.Pattern == "" {
					nullable = true
				}
				lits = append(lits, d.Pattern)
			}
		}
		if nullable {
			nullableSkipped = append(nullableSkipped, f)
			continue
		}
		grammars++
		lg := &loadedGrammar{name: f, grammar: g, compiled: c}
		r := rand.New(rand.NewSource(int64(fi)))
		samples := append([]string{}, generic...)
		samples = append(samples, strings.Join(lits, " "), strings.Join(lits, ""), strings.Join(lits, "\n  "))
		for _, kw := range g.Keywords {
			samples = append(samples, kw+" "+strings.ToUpper(kw)+" x"+kw)
		}
		bad := 0
		for _, s := range samples {
			for i := 0; i <= mutations && bad < 2; i++ {
				src := s
				if i > 0 {
					src = mutate(r, s)
				}
				comparisons++
				if err := compareOne(lg, src); err != nil {
					t.Errorf("%s: %v\ninput: %q", f, err, src)
					bad++
				}
			}
		}
	}
	if grammars < 20 && stride == 1 {
		t.Errorf("only %d grammars compared; expected many more", grammars)
	}
	for _, f := range nullableSkipped {
		t.Errorf("grammar %s has a rule that matches the empty string and was left out of the differential", f)
	}
	t.Logf("skipped %d nullable grammars", len(nullableSkipped))
	t.Logf("compared %d grammars, %d comparisons", grammars, comparisons)
}

// TestDualRunIsOnAndCatchesDisagreement proves the switch is live and that a
// wrong matcher would be noticed rather than silently trusted.
func TestDualRunIsOnAndCatchesDisagreement(t *testing.T) {
	if !dualRunMatchers {
		t.Skip("benchmarks requested; dual-run disabled")
	}
	c, err := Compile(mustGrammar(t, "NAME = /[a-z]+/\n"))
	if err != nil {
		t.Fatal(err)
	}
	r := c.tables["default"].byByte['a'][0]
	if r.m == nil {
		t.Fatal("expected a matcher")
	}
	if n := r.matchLen("abc1"); n != 3 {
		t.Fatalf("matchLen = %d", n)
	}
	// Sabotage the matcher: make it demand an extra character.
	r.m.steps[0].min = 5
	defer func() {
		r.m.steps[0].min = 1
		if recover() == nil {
			t.Error("a disagreeing matcher must panic under dual-run")
		}
	}()
	r.matchLen("abc1")
}
