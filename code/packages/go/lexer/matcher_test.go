package lexer

// Tests for the deterministic matcher (matcher.go) and the first-byte
// analysis (compiled_lexer.go: firstByteSet).
//
// The matcher's whole claim is "same answer as the regexp engine". So the
// central tests compare it with regexp on tables, on randomly generated
// patterns and inputs, and under `go test -fuzz`.

import (
	"math/rand"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// regexpLen is the reference: what the legacy engine computed for a pattern at
// the start of s (-1 for no match).
func regexpLen(t testing.TB, pattern, s string) (int, bool) {
	re, err := regexp.Compile("^" + pattern)
	if err != nil {
		return 0, false
	}
	loc := re.FindStringIndex(s)
	if loc == nil || loc[0] != 0 {
		return -1, true
	}
	return loc[1], true
}

func TestMatcherBuildsForCommonShapes(t *testing.T) {
	shapes := []string{
		`[A-Za-z_][A-Za-z0-9_]*`,
		`-?[0-9]+`,
		`0x[0-9a-fA-F]+`,
		`[A-Za-z0-9_]+<[^>\n]*>`,
		`"([^"\\]|\\.)*"(\.\.\.)?`,
		`"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"`,
		`-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*`,
		`[ \t\r\n]+`,
		`//[^\n]*`,
		`/\*([^*]|\*[^/])*\*/`,
		`#.*`,
		`[0-9]{2,4}`,
		`(?:ab|cd)+`,
		`<\.\.\. [a-z_0-9]+ resumed>`,
	}
	for _, p := range shapes {
		m := buildMatcher(p)
		if m == nil {
			// `\*[^/]` alternatives overlap (`*` is also in [^*]? no, but
			// the comment shape is allowed to fall back); only insist on the
			// ones that matter for speed.
			if p == `/\*([^*]|\*[^/])*\*/` {
				continue
			}
			t.Errorf("expected a matcher for %q", p)
		}
	}
}

func TestMatcherRefusesUnsafeShapes(t *testing.T) {
	unsafe := []string{
		`[a-z]*a`,              // greedy run would swallow the final a: needs backtracking
		`(?:a|ab)*c`,           // overlapping alternatives
		`(?:ab|a)+b`,           // overlapping alternatives
		`é`,                    // fine as a literal, but see below: single literal handled elsewhere
		`[é]+`,                 // partial non-ASCII class
		`\bfoo`,                // zero-width assertion
		`^a`,                   // explicit anchor
		`a$`,                   // explicit anchor
		`a+?`,                  // non-greedy
		`(a*)*`,                // nested repetition
		`(?:(?:ab)*c)*`,        // nested loop
		`(?i)s`,                // case folding reaches non-ASCII (U+017F)
		`a{2000}`,              // too large to unroll
		`(a|)`,                 // empty branch
		`[a-z]+[a-z]`,          // run followed by overlapping class
		`(?:a|b)*[ab]c`,        // loop followed by overlapping class
		``,                     // empty pattern
		`\x{FFFD}`,             // would also match invalid UTF-8 bytes
		`(?:x(?:b|c)*|y)+`,     // loop inside a branch is not supported
		`(?:ab|ac)`,            // branches share a prefix up to the end of the shorter... (not exclusive? checked below)
		`(?:a*|b)`,             // nullable alternation (not a loop of branches)
		`[^\x00-\x{10FFFF}]+x`, // empty class
	}
	for _, p := range unsafe {
		if p == `é` || p == `(?:ab|ac)` || p == `(?:a*|b)` || p == `[^\x00-\x{10FFFF}]+x` {
			continue // covered by their own table tests; may or may not build
		}
		if m := buildMatcher(p); m != nil {
			t.Errorf("matcher unexpectedly built for %q", p)
		}
	}
}

func TestMatcherSingleLiteralBecomesLiteralRule(t *testing.T) {
	g := mustGrammar(t, "A = /<unfinished \\.\\.\\.>/\nB = \"x\"\nC = /[a-z]+/\n")
	c, err := Compile(g)
	if err != nil {
		t.Fatal(err)
	}
	rules := c.tables["default"].byByte['<']
	if len(rules) == 0 || rules[0].literal != "<unfinished ...>" || rules[0].m != nil {
		t.Errorf("regex that is a plain string should use the literal path, got %+v", rules)
	}
	rules = c.tables["default"].byByte['a']
	if len(rules) == 0 || rules[0].m == nil {
		t.Errorf("simple class run should use a matcher")
	}
}

var matcherPatterns = []string{
	`[A-Za-z_][A-Za-z0-9_]*`,
	`-?[0-9]+`,
	`0x[0-9a-fA-F]+`,
	`[A-Za-z0-9_]+<[^>\n]*>`,
	`"([^"\\]|\\.)*"(\.\.\.)?`,
	`"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"`,
	`-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*`,
	`[ \t\r\n]+`,
	`//[^\n]*`,
	`#.*`,
	`(?i)select`,
	`[0-9]{2,4}`,
	`(?:ab|cd)+`,
	`a*`,
	`(?:a|b)*`,
	`é+`,
	`[^a]+`,
	`.`,
	`(?s).+`,
	`x?y?z?`,
	`\x{D800}[a-z]*`,
	`[a-z]+\x{DFFF}`,
	`\x{D800}`,
	`(?:\x{D800}|a)+`,
}

var matcherInputs = []string{
	"", "a", "abc", "_x9 rest", "-12 rest", "-", "-x", "0x1Fg", "0x", "name<a b>tail",
	"name<unterminated", "\"hi\"", "\"hi\"...", "\"hi\"..", "\"a\\\"b\"", "\"a\\", "\"\\u00e9\"",
	"\"\\u00e\"", "\"é日本\"x", "\"\xff\xfe\"", "-12.50e+3,", "1.", "1e", "  \t\r\n x", "//c\nx",
	"# c é\n", "SELECT", "SeLeCt x", "1234567", "12", "1", "ababcd", "abcd", "é", "éé", "a\nb",
	"\n", "\xff", "\xc3", "aaaa", "xyz", "xz", "zz", "ab", "ba",
	"\uFFFDabc", "abc\uFFFD", "\uFFFD", "\xed\xa0\x80abc", "abc\xed\xbf\xbf",
}

func TestMatcherMatchesRegexpTable(t *testing.T) {
	built := 0
	for _, p := range matcherPatterns {
		m := buildMatcher(p)
		if m == nil {
			continue
		}
		built++
		for _, s := range matcherInputs {
			want, ok := regexpLen(t, p, s)
			if !ok {
				t.Fatalf("pattern %q does not compile", p)
			}
			if got := m.match(s); got != want {
				t.Errorf("pattern %q on %q: matcher %d regexp %d", p, s, got, want)
			}
		}
	}
	if built < 10 {
		t.Fatalf("only %d patterns built a matcher; the table is not exercising it", built)
	}
}

// randomPattern builds a pattern from pieces the matcher understands (and a
// few it should refuse), so both outcomes are exercised.
func randomPattern(r *rand.Rand, depth int) string {
	atoms := []string{`a`, `b`, `ab`, `x`, `\.`, `é`, `[a-c]`, `[^a]`, `[ab]`, `.`, `[0-9]`, `\n`, `[^\n]`, `-`}
	quants := []string{``, ``, ``, `*`, `+`, `?`, `{2}`, `{1,3}`, `{0,2}`}
	var sb strings.Builder
	n := 1 + r.Intn(4)
	for i := 0; i < n; i++ {
		var piece string
		if depth > 0 && r.Intn(4) == 0 {
			k := 1 + r.Intn(3)
			alts := make([]string, k)
			for j := range alts {
				alts[j] = randomPattern(r, depth-1)
			}
			piece = `(?:` + strings.Join(alts, `|`) + `)`
		} else {
			piece = atoms[r.Intn(len(atoms))]
		}
		sb.WriteString(piece)
		sb.WriteString(quants[r.Intn(len(quants))])
	}
	return sb.String()
}

func randomInput(r *rand.Rand) string {
	alphabet := []string{"a", "b", "c", "x", ".", "é", "\n", "0", "-", "\xff", "ab", " "}
	n := r.Intn(8)
	var sb strings.Builder
	for i := 0; i < n; i++ {
		sb.WriteString(alphabet[r.Intn(len(alphabet))])
	}
	return sb.String()
}

func TestMatcherMatchesRegexpRandom(t *testing.T) {
	r := rand.New(rand.NewSource(42))
	built, total := 0, 0
	iterations := 20000
	if testing.Short() {
		iterations = 3000
	}
	for i := 0; i < iterations; i++ {
		p := randomPattern(r, 2)
		if _, ok := regexpLen(t, p, ""); !ok {
			continue
		}
		total++
		m := buildMatcher(p)
		if m == nil {
			continue
		}
		built++
		for k := 0; k < 12; k++ {
			s := randomInput(r)
			want, _ := regexpLen(t, p, s)
			if got := m.match(s); got != want {
				t.Fatalf("pattern %q on %q: matcher %d regexp %d", p, s, got, want)
			}
		}
	}
	if built < total/10 {
		t.Errorf("only %d of %d random patterns built a matcher; generator too hostile", built, total)
	}
	t.Logf("random patterns: %d compiled, %d built a matcher", total, built)
}

func TestFirstByteSetIsASuperset(t *testing.T) {
	r := rand.New(rand.NewSource(7))
	for i := 0; i < 8000; i++ {
		p := randomPattern(r, 2)
		wrapped := "^(?:" + p + ")"
		re, err := regexp.Compile(wrapped)
		if err != nil {
			continue
		}
		set := firstByteSet(wrapped)
		for k := 0; k < 10; k++ {
			s := randomInput(r)
			loc := re.FindStringIndex(s)
			if loc == nil {
				continue
			}
			if loc[1] == 0 {
				if set != nil {
					t.Fatalf("pattern %q matches empty but first-set is not 'any'", p)
				}
				continue
			}
			if set != nil && !set[s[0]] {
				t.Fatalf("pattern %q matched %q but first byte %#x is not in the set", p, s[:loc[1]], s[0])
			}
		}
	}
}

func TestFirstByteSetShapes(t *testing.T) {
	cases := []struct {
		pattern string
		has     []byte
		hasNot  []byte
		any     bool
	}{
		{`abc`, []byte("a"), []byte("bc"), false},
		{`[a-c]x`, []byte("abc"), []byte("xd"), false},
		{`a|b`, []byte("ab"), []byte("c"), false},
		{`a*b`, []byte("ab"), []byte("c"), false},
		{`a+`, []byte("a"), []byte("b"), false},
		{`(?i)k`, []byte("kK"), []byte("a"), false},
		{`é`, []byte{0xc3}, []byte("a"), false},
		{`\x{FFFD}`, []byte{0x80, 0xff}, []byte("a"), false},
		{`.`, []byte("a\x80"), []byte("\n"), false},
		{`(?s).`, []byte("a\n"), nil, false},
		{`a*`, nil, nil, true},
		{`\bfoo`, []byte("f"), []byte("o"), false},
		{`(a)(b)`, []byte("a"), []byte("b"), false},
		{`[^a]`, []byte("b\xff"), []byte("a"), false},
		{`a{0,2}b`, []byte("ab"), []byte("c"), false},
		{`(?:)`, nil, nil, true},
	}
	for _, c := range cases {
		set := firstByteSet("^(?:" + c.pattern + ")")
		if c.any {
			if set != nil {
				t.Errorf("%q: expected any", c.pattern)
			}
			continue
		}
		if set == nil {
			t.Errorf("%q: unexpected any", c.pattern)
			continue
		}
		for _, b := range c.has {
			if !set[b] {
				t.Errorf("%q: byte %#x missing", c.pattern, b)
			}
		}
		for _, b := range c.hasNot {
			if set[b] {
				t.Errorf("%q: byte %#x should not be present", c.pattern, b)
			}
		}
	}
	if firstByteSet("(") != nil {
		t.Error("an invalid pattern must yield 'any'")
	}
}

// FuzzMatcherMatchesRegexp lets the fuzzer invent patterns and inputs. Whenever
// buildMatcher accepts a pattern, its answer must equal the regexp engine's,
// and the first-byte set must cover the first byte of any non-empty match.
func FuzzMatcherMatchesRegexp(f *testing.F) {
	for _, p := range matcherPatterns {
		for _, s := range matcherInputs {
			f.Add(p, s)
		}
	}
	// Seed with every matcher-eligible pattern from the real grammars.
	for _, p := range grammarPatterns() {
		if buildMatcher(p) != nil {
			for _, in := range []string{"", "abc", "\"a\\\"b\"x", "é\xff", "-12.5e+3 ", "name<x>"} {
				f.Add(p, in)
			}
		}
	}
	f.Fuzz(func(t *testing.T, pattern, input string) {
		checkMatcherAgainstRegexp(t, pattern, input)
	})
}

// Surrogate code points parse as literal runes in a pattern but can never equal
// any input rune, and encoding one as a string yields U+FFFD. A matcher that
// took the string form would accept text the regexp engine refuses.
func TestMatcherRefusesSurrogateLiterals(t *testing.T) {
	for _, p := range []string{`\x{D800}`, `\x{D800}[a-z]*`, `[a-z]+\x{DFFF}`, `(?:\x{DBFF}|a)+`, `[\x{D800}-\x{DFFF}]+`} {
		if m := buildMatcher(p); m != nil {
			t.Errorf("matcher built for surrogate pattern %q", p)
		}
	}
}

// Regression: both of these were accepted by Compiled although the legacy
// engine rejected the input.
func TestSurrogateLiteralRegressions(t *testing.T) {
	cases := []struct{ tokens, input string }{
		{"X = /\\x{D800}[a-z]*/\n", "\uFFFDabc"},
		{"X = /[a-z]+\\x{DFFF}/\n", "abc\uFFFD"},
		{"X = /\\x{D800}/\n", "\uFFFD"},
		{"X = /\\x{D800}/\n", "\xff"},
	}
	for _, c := range cases {
		g := mustGrammar(t, c.tokens)
		assertEquivalent(t, g, c.input, "abc", "")
	}
}

// ---------------------------------------------------------------------------
// Structured fuzzing
// ---------------------------------------------------------------------------

// patternFromBytes decodes a byte string into a pattern built from atoms,
// quantifiers, flags and groups, so a mutating fuzzer explores pattern
// *structure* (mutating raw pattern text mostly produces syntax errors).
func patternFromBytes(b []byte) string {
	atoms := []string{
		`a`, `b`, `ab`, `x`, `\.`, `é`, `日`, `[a-c]`, `[^a]`, `[ab]`, `.`, `[0-9]`, `\n`, `[^\n]`, `-`,
		`\x{D800}`, `\x{DFFF}`, `\x{FFFD}`, `[\x{80}-\x{10FFFF}]`, `[^"\\]`, `\\`, `"`, `[a-zA-Z_]`,
		`\w`, `\s`, `\d`, `\b`, `^`, `$`, `(?i:k)`, `[é]`, `\Q.\E`,
	}
	quants := []string{``, ``, `*`, `+`, `?`, `{2}`, `{1,3}`, `{0,2}`, `*?`, `+?`}
	flags := []string{``, ``, ``, `(?i)`, `(?s)`, `(?m)`, `(?U)`}
	pos := 0
	next := func(n int) int {
		if pos >= len(b) {
			return 0
		}
		v := int(b[pos]) % n
		pos++
		return v
	}
	var build func(depth int) string
	build = func(depth int) string {
		var sb strings.Builder
		n := 1 + next(4)
		for i := 0; i < n; i++ {
			if depth > 0 && next(4) == 0 {
				k := 1 + next(3)
				alts := make([]string, k)
				for j := range alts {
					alts[j] = build(depth - 1)
				}
				sb.WriteString(`(?:` + strings.Join(alts, `|`) + `)`)
			} else {
				sb.WriteString(atoms[next(len(atoms))])
			}
			sb.WriteString(quants[next(len(quants))])
		}
		return sb.String()
	}
	return flags[next(len(flags))] + build(2)
}

// FuzzMatcherStructured fuzzes the matcher and first-byte analysis with
// patterns decoded from a byte string (see patternFromBytes).
func FuzzMatcherStructured(f *testing.F) {
	for i := 0; i < 40; i++ {
		seed := make([]byte, 12)
		for j := range seed {
			seed[j] = byte(i*31 + j*7)
		}
		for _, in := range []string{"", "abc", "\uFFFDa", "é日x", "\xff", "-a\n.\""} {
			f.Add(seed, in)
		}
	}
	f.Fuzz(func(t *testing.T, choices []byte, input string) {
		checkMatcherAgainstRegexp(t, patternFromBytes(choices), input)
	})
}

// checkMatcherAgainstRegexp is the property shared by the fuzz targets.
func checkMatcherAgainstRegexp(t *testing.T, pattern, input string) {
	t.Helper()
	// The reference is the legacy shape: `^pattern`, match must start at 0.
	legacy, err := regexp.Compile("^" + pattern)
	if err != nil {
		return
	}
	loc := legacy.FindStringIndex(input)
	if loc != nil && loc[0] != 0 {
		loc = nil
	}
	if m := buildMatcher(pattern); m != nil {
		want := -1
		if loc != nil {
			want = loc[1]
		}
		if got := m.match(input); got != want {
			t.Fatalf("pattern %q on %q: matcher %d regexp %d", pattern, input, got, want)
		}
	}
	wrapped := "^(?:" + pattern + ")"
	if _, werr := regexp.Compile(wrapped); werr != nil {
		return // the engine falls back to the legacy form for these
	}
	if set := firstByteSet(wrapped); loc != nil && set != nil {
		if loc[1] == 0 {
			t.Fatalf("pattern %q matches empty on %q but first-set is not 'any'", pattern, input)
		}
		if !set[input[0]] {
			t.Fatalf("pattern %q matched %q; first byte %#x not in first-set", pattern, input[:loc[1]], input[0])
		}
	}
}

func TestMatcherStructuredRandom(t *testing.T) {
	r := rand.New(rand.NewSource(99))
	n := 2500
	if testing.Short() {
		n = 500
	}
	built := 0
	inputs := []string{"", "a", "ab", "abc", "-a", "é", "日x", "\uFFFDa", "\xff", "a\nb", "aaaa", "A", "k", "K"}
	for i := 0; i < n; i++ {
		b := make([]byte, 6+r.Intn(20))
		r.Read(b)
		p := patternFromBytes(b)
		if buildMatcher(p) != nil {
			built++
		}
		for _, in := range inputs {
			checkMatcherAgainstRegexp(t, p, in)
		}
	}
	if built == 0 {
		t.Error("structured generator never produced a matcher-eligible pattern")
	}
}

// grammarPatterns returns every regex pattern of every grammar under
// code/grammars (for fuzz seeding).
func grammarPatterns() []string {
	files, _ := filepath.Glob("../../../grammars/*/*.tokens")
	seen := map[string]bool{}
	var out []string
	for _, f := range files {
		data, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		g, err := grammartools.ParseTokenGrammar(string(data))
		if err != nil {
			continue
		}
		for _, d := range allDefinitions(g) {
			if d.IsRegex && !seen[d.Pattern] {
				seen[d.Pattern] = true
				out = append(out, d.Pattern)
			}
		}
	}
	return out
}

// TestMatcherOnEveryGrammarPattern checks each matcher-eligible pattern from
// the real grammars against regexp on a spread of inputs, and logs how many
// patterns the matcher covers.
func TestMatcherOnEveryGrammarPattern(t *testing.T) {
	built, total := 0, 0
	for _, p := range grammarPatterns() {
		total++
		if buildMatcher(p) == nil {
			continue
		}
		built++
		for _, in := range matcherInputs {
			checkMatcherAgainstRegexp(t, p, in)
		}
	}
	t.Logf("grammar patterns: %d, matcher-eligible: %d", total, built)
}
