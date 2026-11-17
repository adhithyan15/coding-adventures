package straceparser

import (
	"errors"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	stracelexer "github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer"
)

// compactOptions are the limit sets every equivalence check runs under: the
// defaults, tight depth/token/memo limits (so the limit errors are compared
// too) and lenient quoted strings.
var compactOptions = func() []Options {
	def := DefaultOptions()
	depth := def
	depth.MaxDepth = 12
	tok := def
	tok.MaxTokens = 7
	memo := def
	memo.MaxMemoSlots = 40
	short := def
	short.MaxLineBytes = 30
	len_ := def
	len_.LenientQuoted = true
	// A start rule other than `line`: the tree has no `line` root, so Body is
	// nil in both paths.  Lines are parsed from the body rule, so a call with
	// no pid parses.
	body := def
	body.StartRule = "body"
	pid := def
	pid.StartRule = "pid"
	return []Options{def, depth, tok, memo, short, len_, {}, body, pid}
}()

// sameResult runs one line through the legacy path and through lp (a reused
// LineParser) and reports every difference.
func sameResult(t testing.TB, lp *LineParser, line string, opts Options) {
	t.Helper()
	ast, toks, err := ParseLineTokens(line, opts)
	cerr := lp.ParseLineInto(line, opts)
	if (err == nil) != (cerr == nil) {
		t.Fatalf("%q: legacy err %v, compact err %v", line, err, cerr)
	}
	if err != nil {
		if !reflect.DeepEqual(err, cerr) {
			t.Fatalf("%q: errors differ:\n legacy  %#v\n compact %#v", line, err, cerr)
		}
		if err.Error() != cerr.Error() {
			t.Fatalf("%q: error text differs: %q vs %q", line, err, cerr)
		}
	}
	if !reflect.DeepEqual(toks, lp.Tokens()) {
		t.Fatalf("%q: tokens differ", line)
	}
	if err != nil {
		if lp.BodyKind() != "" || lp.Body() != parser.NoNode || lp.RawArgs() != nil || lp.TextAfterClose() != "" {
			t.Fatalf("%q: a failed parse left a result", line)
		}
		if _, ok := lp.PrefixBeforeUnfinished(); ok {
			t.Fatalf("%q: a failed parse reported a prefix", line)
		}
		return
	}
	if !reflect.DeepEqual(ast, lp.Tree().ToAST()) {
		t.Fatalf("%q: trees differ", line)
	}
	if got, want := lp.BodyKind(), BodyKind(ast); got != want {
		t.Fatalf("%q: BodyKind %q, want %q", line, got, want)
	}
	if got, want := lp.RawArgs(), RawArgs(line, ast); !reflect.DeepEqual(got, want) {
		t.Fatalf("%q: RawArgs %q, want %q", line, got, want)
	}
	// A non-empty dst is extended, never replaced, even when there is nothing
	// to append.
	want := RawArgs(line, ast)
	if got := lp.AppendRawArgs([]string{"pre"}); !reflect.DeepEqual(got, append([]string{"pre"}, want...)) {
		t.Fatalf("%q: AppendRawArgs(dst) %q, want pre + %q", line, got, want)
	}
	if got, want := lp.TextAfterClose(), TextAfterClose(line, ast); got != want {
		t.Fatalf("%q: TextAfterClose %q, want %q", line, got, want)
	}
	gp, gok := lp.PrefixBeforeUnfinished()
	wp, wok := PrefixBeforeUnfinished(line, ast)
	if gp != wp || gok != wok {
		t.Fatalf("%q: PrefixBeforeUnfinished (%q,%v), want (%q,%v)", line, gp, gok, wp, wok)
	}
}

func corpusLines(t testing.TB) []string {
	var all []string
	for _, f := range []string{"testdata/corpus.txt", "testdata/corpus_refused.txt"} {
		all = append(all, lines(t, f)...)
	}
	files, _ := filepath.Glob("testdata/*.strace")
	for _, f := range files {
		all = append(all, lines(t, f)...)
	}
	if len(all) < 500 {
		t.Fatalf("only %d corpus lines", len(all))
	}
	return all
}

// Every corpus, refused-corpus and fixture line, under every option set,
// through ONE reused LineParser (so reuse is exercised too).
func TestCompactEquivalenceCorpus(t *testing.T) {
	var lp LineParser
	for _, l := range corpusLines(t) {
		for _, o := range compactOptions {
			sameResult(t, &lp, l, o)
		}
	}
}

// The same random generator as TestParseNeverPanicsOnRandomInput, compared.
func TestCompactEquivalenceRandom(t *testing.T) {
	alpha := []string{"1  ", "f(", ")", "[", "]", "{", "}", ",", " ", "<", ">", "\"", "\\", "3</x>", "<unfinished ...>", "<... f resumed>",
		"+++ exited with 0 +++", "--- ", " ---", "a", "=", "|", "x=", "\t", "???(", "...", "(", "O_RDONLY", "-1", "\n", "\"a\\n\"", "\\q",
		// non-ASCII spaces and BOM, raw control and high bytes, dead and resumed forms
		"\u00a0", "\u2003", "\u3000", "\u1680", "\ufeff", "\x85", "\xa0", "\xff", "\x00", "\x7f",
		"???( <unfinished ...>", "???()  = ?", "<... ??? resumed>)  = ?", "<... f resumed>)  = 0", "<... resumed>"}
	seed := uint32(11)
	next := func(n int) int {
		seed = seed*1664525 + 1013904223
		return int(seed>>8) % n
	}
	var lp LineParser
	for i := 0; i < 20000; i++ {
		var sb strings.Builder
		if next(8) != 0 {
			sb.WriteString("1  ")
		}
		for k := next(25); k >= 0; k-- {
			sb.WriteString(alpha[next(len(alpha))])
		}
		sameResult(t, &lp, sb.String(), compactOptions[next(len(compactOptions))])
	}
}

func FuzzCompactEquivalence(f *testing.F) {
	for _, l := range corpusLines(f) {
		f.Add(l)
	}
	f.Add("")
	f.Add("1  f(" + strings.Repeat("(", 80))
	var lp LineParser
	f.Fuzz(func(t *testing.T, line string) {
		sameResult(t, &lp, line, DefaultOptions())
		sameResult(t, &lp, line, compactOptions[1])
	})
}

// A reused LineParser: long line, short line, error line, short line again.
// Each result must be that of the line just parsed, with nothing left over.
func TestLineParserReuse(t *testing.T) {
	var lp LineParser
	long := "1  openat(AT_FDCWD</w>, \"/e\", " + strings.Repeat("O_RDONLY|", 4000) + "O_CLOEXEC) = 3</e>"
	short := `1  close(3</e>) = 0`
	bad := `1  close(3</e>`
	seq := []string{long, short, bad, short, long, bad, ""}
	for round := 0; round < 3; round++ {
		for _, l := range seq {
			sameResult(t, &lp, l, DefaultOptions())
		}
	}
	if err := lp.ParseLineInto(short, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	if lp.BodyKind() != "call_line" || lp.TextAfterClose() != " = 0" {
		t.Errorf("short line after errors: %q %q", lp.BodyKind(), lp.TextAfterClose())
	}
	lp.Reset()
	if lp.BodyKind() != "" || lp.Tokens() != nil || lp.Tree().Root() != parser.NoNode {
		t.Error("Reset left a result")
	}
}

// The raw strings outlive the next parse: they are substrings of the line, not
// of any reused buffer.
func TestCompactStringsOutliveNextParse(t *testing.T) {
	var lp LineParser
	line := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`
	if err := lp.ParseLineInto(line, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	args, rest := lp.RawArgs(), lp.TextAfterClose()
	if err := lp.ParseLineInto(`1  +++ exited with 0 +++`, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(args, []string{"AT_FDCWD</w>", `"/e"`, "O_RDONLY"}) || rest != " = 3</e>" {
		t.Errorf("kept results changed: %q %q", args, rest)
	}
}

// With a start rule other than `line` there is no body, as in the legacy Body.
func TestCompactNonLineStartRule(t *testing.T) {
	var lp LineParser
	toks, err := stracelexer.TokenizeLineMax(`1  `, 0)
	if err != nil {
		t.Fatal(err)
	}
	o := DefaultOptions().Options
	o.StartRule = "pid"
	if err := compiled.ParseInto(&lp.tree, toks, o); err != nil {
		t.Fatal(err)
	}
	if lp.findBody() != parser.NoNode {
		t.Error("an args-rooted tree reported a body")
	}
	// A `line` root with no body child (not producible by the grammar, but
	// findBody must not assume it): parse a pid alone is refused by the lexer,
	// so the empty-tree case stands for it.
	lp.Reset()
	if lp.findBody() != parser.NoNode {
		t.Error("an empty tree reported a body")
	}
}

func TestLineParserNil(t *testing.T) {
	var lp *LineParser
	if err := lp.ParseLineInto("1  f()", Options{}); err == nil {
		t.Error("nil LineParser accepted")
	}
	lp.Reset()
	if lp.Tree() != nil || lp.Tokens() != nil || lp.Body() != parser.NoNode || lp.BodyKind() != "" ||
		lp.RawArgs() != nil || lp.TextAfterClose() != "" {
		t.Error("nil LineParser returned a non-zero value")
	}
	if _, ok := lp.PrefixBeforeUnfinished(); ok {
		t.Error("nil LineParser reported a prefix")
	}
	if got := lp.AppendRawArgs([]string{"x"}); len(got) != 1 {
		t.Errorf("nil LineParser changed dst: %q", got)
	}
}

// Options.PostParse hooks transform *ASTNodes, which this path never builds:
// refused (KindSyntax), where the legacy path runs them.
func TestCompactRefusesPostParse(t *testing.T) {
	var lp LineParser
	o := DefaultOptions()
	o.PostParse = []parser.PostParseHook{func(a *parser.ASTNode) *parser.ASTNode { return a }}
	line := `1  close(3</e>) = 0`
	if _, _, err := ParseLineTokens(line, o); err != nil {
		t.Fatalf("legacy refused PostParse: %v", err)
	}
	err := lp.ParseLineInto(line, o)
	var le *LineError
	if !errors.As(err, &le) || le.Kind != KindSyntax || !strings.Contains(err.Error(), "PostParse") {
		t.Fatalf("compact: %v, want a KindSyntax PostParse refusal", err)
	}
	if lp.BodyKind() != "" {
		t.Error("a refused line left a result")
	}
}

// A caller who resets the tree behind the LineParser's back gets empty
// results, not a panic.
func TestCompactTreeResetBehindBack(t *testing.T) {
	var lp LineParser
	if err := lp.ParseLineInto(`1  openat(AT_FDCWD</w>, "/e" <unfinished ...>`, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	lp.Tree().Reset()
	if lp.Body() != parser.NoNode || lp.BodyKind() != "" || lp.RawArgs() != nil || lp.TextAfterClose() != "" {
		t.Error("helpers returned results from a reset tree")
	}
	if _, ok := lp.PrefixBeforeUnfinished(); ok {
		t.Error("prefix from a reset tree")
	}
	sameResult(t, &lp, `1  close(3</e>) = 0`, DefaultOptions()) // and it recovers
}

// Dead lines are single tokens: no ")" to cut after.
func TestTextAfterCloseDeadLines(t *testing.T) {
	for _, l := range []string{`1  ???( <unfinished ...>`, `1  ???()  = ?`, `1  <... ??? resumed>)  = ?`} {
		ast, err := ParseLine(l, DefaultOptions())
		if err != nil {
			t.Fatalf("%q: %v", l, err)
		}
		if got := TextAfterClose(l, ast); got != "" {
			t.Errorf("%q: %q", l, got)
		}
	}
}

// Many lines through ONE LineParser, with occasional Resets between them,
// each compared with the legacy path.  Input bytes are split on 0x01 (up to 9
// lines) so the fuzzer controls the sequence; Reset happens where a line
// starts with 0x02.
func FuzzCompactSequence(f *testing.F) {
	all := corpusLines(f)
	for i := 0; i+8 < len(all); i += 37 {
		f.Add(strings.Join(all[i:i+9], "\x01"))
	}
	f.Add("1  f(" + "\x01" + "\x02" + "1  g()")
	var lp LineParser
	f.Fuzz(func(t *testing.T, in string) {
		for i, l := range strings.SplitN(in, "\x01", 9) {
			if strings.HasPrefix(l, "\x02") {
				lp.Reset()
				l = l[1:]
			}
			sameResult(t, &lp, l, compactOptions[(len(l)+i)%len(compactOptions)])
		}
	})
}

// A steady-state reuse allocates only what the lexer does (its token slice and
// the strings it keeps); the parse and the helpers add nothing.  Pin that by
// comparing against the lexer alone with a loose bound, so a regression to a
// tree-building path (hundreds of allocations) is caught.
func TestCompactAllocsBounded(t *testing.T) {
	var lp LineParser
	line := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY|O_CLOEXEC) = 3</e>`
	buf := make([]string, 0, 16)
	run := func() {
		if err := lp.ParseLineInto(line, DefaultOptions()); err != nil {
			t.Fatal(err)
		}
		buf = lp.AppendRawArgs(buf[:0])
		_ = lp.TextAfterClose()
		_, _ = lp.PrefixBeforeUnfinished()
	}
	run()
	compact := testing.AllocsPerRun(100, run)
	legacy := testing.AllocsPerRun(100, func() {
		ast, _, _ := ParseLineTokens(line, DefaultOptions())
		_ = RawArgs(line, ast)
	})
	if compact >= legacy/2 {
		t.Errorf("compact allocs %.0f not well below legacy %.0f", compact, legacy)
	}
}

// ---------------------------------------------------------------------------
// Benchmarks: legacy vs compact on the fixture corpus
// ---------------------------------------------------------------------------

// BenchmarkLegacyFullFixtures is ParseLineTokens plus the helpers a consumer
// calls (BodyKind, RawArgs, TextAfterClose, PrefixBeforeUnfinished).
func BenchmarkLegacyFullFixtures(b *testing.B) {
	ls, total := benchLines(b)
	opts := DefaultOptions()
	b.SetBytes(int64(total))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, l := range ls {
			ast, _, err := ParseLineTokens(l, opts)
			if err != nil {
				b.Fatal(err)
			}
			_ = BodyKind(ast)
			_ = RawArgs(l, ast)
			_ = TextAfterClose(l, ast)
			_, _ = PrefixBeforeUnfinished(l, ast)
		}
	}
}

// BenchmarkCompactFullFixtures is the same work through a reused LineParser.
func BenchmarkCompactFullFixtures(b *testing.B) {
	ls, total := benchLines(b)
	opts := DefaultOptions()
	var lp LineParser
	buf := make([]string, 0, 32)
	b.SetBytes(int64(total))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, l := range ls {
			if err := lp.ParseLineInto(l, opts); err != nil {
				b.Fatal(err)
			}
			_ = lp.BodyKind()
			buf = lp.AppendRawArgs(buf[:0])
			_ = lp.TextAfterClose()
			_, _ = lp.PrefixBeforeUnfinished()
		}
	}
}

// BenchmarkCompactParseOnlyFixtures is the compact parse with no helpers, to
// compare with BenchmarkLexParseFixtures.
func BenchmarkCompactParseOnlyFixtures(b *testing.B) {
	ls, total := benchLines(b)
	opts := DefaultOptions()
	var lp LineParser
	b.SetBytes(int64(total))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, l := range ls {
			if err := lp.ParseLineInto(l, opts); err != nil {
				b.Fatal(err)
			}
		}
	}
}
