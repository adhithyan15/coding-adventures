package lexer

import (
	"math/rand"
	"os"
	"regexp"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// asciiRunPatterns exercises the same repetition shapes used by the strace
// token grammar, plus a high-rune class which must stay on unit.at.
var asciiRunPatterns = []string{
	`[a-z]*!`,
	`[a-z]+!`,
	`[a-z]{0}!`,
	`[a-z]{0,3}!`,
	`[a-z]{2,4}!`,
	`[0-9]{1,64}x`,
	`[a-z0-9_]{1,64}\(`,
	`[A-Za-z0-9_-]+<`,
	` +x`,
	`[^xX!]+!`,
}

// legacyMatcherMatch is the pre-specialization matcher loop. Keep this copy
// simple and intentionally use unit.at for every repeated character so the
// optimized path has an independent in-package behavioral reference.
func legacyMatcherMatch(m *matcher, s string) int {
	pos := 0
	for i := range m.steps {
		st := &m.steps[i]
		switch st.kind {
		case stepLit:
			if len(s)-pos < len(st.lit) || s[pos:pos+len(st.lit)] != st.lit {
				return -1
			}
			pos += len(st.lit)
		case stepClass:
			w := st.u.at(s, pos)
			if w == 0 {
				return -1
			}
			pos += w
		case stepRun:
			count := 0
			for st.max < 0 || count < st.max {
				w := st.u.at(s, pos)
				if w == 0 {
					break
				}
				pos += w
				count++
			}
			if count < st.min {
				return -1
			}
		case stepLoop:
			count := 0
			for st.max < 0 || count < st.max {
				if pos >= len(s) {
					break
				}
				np := -1
				for _, bi := range st.dispatch[s[pos]] {
					if p, ok := runBranch(st.branches[bi], s, pos); ok {
						np = p
						break
					}
				}
				if np < 0 {
					break
				}
				pos = np
				count++
			}
			if count < st.min {
				return -1
			}
		}
	}
	return pos
}

type asciiRunOracle struct {
	pattern string
	matcher *matcher
	regexp  *regexp.Regexp
}

func newAsciiRunOracle(t testing.TB, pattern string) asciiRunOracle {
	t.Helper()
	m := buildMatcher(pattern)
	if m == nil {
		t.Fatalf("expected supported matcher for %q", pattern)
	}
	r, err := regexp.Compile("^" + pattern)
	if err != nil {
		t.Fatalf("reference regexp for %q: %v", pattern, err)
	}
	return asciiRunOracle{pattern: pattern, matcher: m, regexp: r}
}

func compareAsciiRunReferences(t *testing.T, oracle asciiRunOracle, input string) {
	t.Helper()
	want := -1
	if loc := oracle.regexp.FindStringIndex(input); loc != nil {
		want = loc[1]
	}
	if got := legacyMatcherMatch(oracle.matcher, input); got != want {
		t.Fatalf("frozen matcher for %q on %q: got %d want %d", oracle.pattern, input, got, want)
	}
	if got := oracle.matcher.match(input); got != want {
		t.Fatalf("optimized matcher for %q on %q: got %d want %d", oracle.pattern, input, got, want)
	}
}

func TestStraceGrammarHasAsciiRunSteps(t *testing.T) {
	data, err := os.ReadFile("../../../grammars/strace/strace.tokens")
	if err != nil {
		t.Fatal(err)
	}
	g, err := grammartools.ParseTokenGrammar(string(data))
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{
		"SYSCALL": false,
		"FUNC":    false,
		"PID":     false,
		"PID_TAG": false,
		"WORD":    false,
		"SPACES":  false,
	}
	for _, def := range allDefinitions(g) {
		if _, selected := want[def.Name]; !selected {
			continue
		}
		compiled, err := compileRule(def, "strace class test")
		if err != nil {
			t.Fatalf("compile %s pattern %q: %v", def.Name, def.Pattern, err)
		}
		if compiled.m == nil {
			t.Fatalf("strace rule %s did not use the deterministic matcher", def.Name)
		}
		for _, st := range compiled.m.steps {
			if st.kind == stepRun && !st.u.high {
				want[def.Name] = true
			}
		}
	}
	for name, found := range want {
		if !found {
			t.Errorf("strace rule %s has no ASCII-only repeated-class step", name)
		}
	}
}

func TestAsciiRunMatchesLegacyAndRegexpExhaustively(t *testing.T) {
	inputs := []string{
		"", "!", "a!", "abcd!", "abcde!", "aX!", "0x", "123456789x", " ", "  x", "   y",
		"0", "12", "1234", "12345", "read(", "clone3(", "foo<", "fooX", "é!", "\xff!", "\xc3!",
		"\xc3\xa9!", "\xe2\x28\xa1!", "abc\xff!", "abc\x80!", "A!", "\x7f!",
		strings.Repeat("1", 63) + "x",
		strings.Repeat("1", 64) + "x",
		strings.Repeat("1", 65) + "x",
	}
	for _, pattern := range asciiRunPatterns {
		oracle := newAsciiRunOracle(t, pattern)
		for _, input := range inputs {
			compareAsciiRunReferences(t, oracle, input)
		}
	}

	// Exhaust every byte string of length zero through four over a compact
	// alphabet containing delimiters, members, valid UTF-8 bytes, and invalid
	// high bytes. This is 30,941 inputs per pattern across the ten patterns.
	alphabet := []byte{'a', 'b', '0', '1', '!', 'x', 'X', '<', ' ', 0x80, 0xc3, 0xa9, 0xff}
	var visit func([]byte, int)
	for _, pattern := range asciiRunPatterns {
		oracle := newAsciiRunOracle(t, pattern)
		visit = func(prefix []byte, remaining int) {
			compareAsciiRunReferences(t, oracle, string(prefix))
			if remaining == 0 {
				return
			}
			for _, b := range alphabet {
				visit(append(prefix, b), remaining-1)
			}
		}
		visit(nil, 4)
	}
}

func TestAsciiRunMatchesLegacyAndRegexpDeterministically(t *testing.T) {
	r := rand.New(rand.NewSource(0x475231))
	alphabet := []byte{'a', 'b', '0', '1', '!', 'x', 'X', '<', ' ', 0x80, 0xc3, 0xa9, 0xff}
	oracles := make([]asciiRunOracle, 0, len(asciiRunPatterns))
	for _, pattern := range asciiRunPatterns {
		oracles = append(oracles, newAsciiRunOracle(t, pattern))
	}
	for i := 0; i < 2048; i++ {
		input := make([]byte, r.Intn(65))
		for j := range input {
			input[j] = alphabet[r.Intn(len(alphabet))]
		}
		compareAsciiRunReferences(t, oracles[r.Intn(len(oracles))], string(input))
	}
}

func TestAsciiRunHugeBoundDoesNotOverflow(t *testing.T) {
	m := buildMatcher(`[a]+!`)
	if m == nil || len(m.steps) != 2 || m.steps[0].kind != stepRun {
		t.Fatal("expected a run followed by a literal")
	}
	m.steps[0].max = int(^uint(0) >> 1)
	oracle, err := regexp.Compile(`^[a]+!`)
	if err != nil {
		t.Fatal(err)
	}
	for _, input := range []string{"", "!", "a!", "aaaa!", "b!", "aaaa"} {
		want := -1
		if loc := oracle.FindStringIndex(input); loc != nil {
			want = loc[1]
		}
		if got := legacyMatcherMatch(m, input); got != want {
			t.Errorf("frozen matcher on %q: got %d want %d", input, got, want)
		}
		if got := m.match(input); got != want {
			t.Errorf("optimized matcher on %q: got %d want %d", input, got, want)
		}
	}
}

func TestAsciiRunUnicodeShapesKeepFallbackSemantics(t *testing.T) {
	high := buildMatcher(`[^xX!]+!`)
	if high == nil || len(high.steps) == 0 || high.steps[0].kind != stepRun || !high.steps[0].u.high {
		t.Fatalf("non-ASCII class must remain on the Unicode matcher path")
	}
	pattern := `[é]+X`
	if buildMatcher(pattern) != nil {
		t.Fatalf("partial Unicode class unexpectedly used deterministic matcher")
	}
	rule, err := compileRule(structTokenDefinition(pattern), "Unicode fallback test")
	if err != nil {
		t.Fatal(err)
	}
	if rule.m != nil || rule.re == nil {
		t.Fatalf("partial Unicode class should use regexp fallback: matcher=%v regexp=%v", rule.m, rule.re)
	}
	re, err := regexp.Compile("^" + pattern)
	if err != nil {
		t.Fatal(err)
	}
	for _, input := range []string{"éX", "ééX", "eX", "\xffX", "\xc3X", "éY"} {
		want := -1
		if loc := re.FindStringIndex(input); loc != nil {
			want = loc[1]
		}
		got := -1
		if loc := rule.re.FindStringIndex(input); loc != nil {
			got = loc[1]
		}
		if got != want {
			t.Errorf("fallback on %q: got %d want %d", input, got, want)
		}
	}
}

func structTokenDefinition(pattern string) grammartools.TokenDefinition {
	return grammartools.TokenDefinition{Name: "ASCII_RUN_TEST", Pattern: pattern, IsRegex: true}
}

func FuzzAsciiRunMatchesLegacyAndRegexp(f *testing.F) {
	for _, input := range []string{
		"", "!", "a!", "abcd!", "abcde!", "1234x", "12345x", "read(", "foo<", "  x",
		"é!", "\xff!", "\xc3!", "\xc3\xa9!", "\xe2\x28\xa1!", "abc\x80!",
	} {
		f.Add(input)
	}
	oracles := make([]asciiRunOracle, 0, len(asciiRunPatterns))
	for _, pattern := range asciiRunPatterns {
		m := buildMatcher(pattern)
		r := regexp.MustCompile("^" + pattern)
		oracles = append(oracles, asciiRunOracle{pattern: pattern, matcher: m, regexp: r})
	}
	f.Fuzz(func(t *testing.T, input string) {
		for _, oracle := range oracles {
			compareAsciiRunReferences(t, oracle, input)
		}
	})
}

func BenchmarkMatcherStraceAsciiRuns(b *testing.B) {
	cases := []struct {
		name    string
		pattern string
		input   string
	}{
		{"syscall", `[a-z0-9_]{1,64}\(`, "clone3("},
		{"word", `[A-Za-z0-9_-]+`, "AT_FDCWD"},
		{"pid", `[1-9][0-9]{0,8} +`, "123456789  "},
		{"spaces", ` +`, "           "},
	}
	for _, tc := range cases {
		b.Run(tc.name, func(b *testing.B) {
			m := buildMatcher(tc.pattern)
			if m == nil {
				b.Fatalf("pattern %q did not compile", tc.pattern)
			}
			b.ReportAllocs()
			b.SetBytes(int64(len(tc.input)))
			b.ResetTimer()
			for i := 0; i < b.N; i++ {
				if m.match(tc.input) < 0 {
					b.Fatal("input did not match")
				}
			}
		})
	}
}
