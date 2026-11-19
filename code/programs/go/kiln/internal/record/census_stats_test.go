package record

// Tests of the census summary's pure tools (census_stats.go): each statistic,
// the sanitiser, the redactor and the Markdown quoting.  Portable: no file is
// touched, so Windows runs them too.

import (
	"reflect"
	"strings"
	"testing"
	"unicode/utf8"
)

func TestPercentileNearestRank(t *testing.T) {
	ten := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	for _, c := range []struct {
		p    int
		want float64
	}{{50, 5}, {90, 9}, {99, 10}, {100, 10}, {1, 1}} {
		if got := percentile(ten, c.p); got != c.want {
			t.Errorf("p%d of 1..10 = %v, want %v", c.p, got, c.want)
		}
	}
	if percentile(nil, 50) != 0 {
		t.Error("an empty slice must give 0")
	}
	if percentile([]float64{42}, 99) != 42 || percentile([]float64{42}, 1) != 42 {
		t.Error("one sample is every percentile")
	}
}

func TestDistOf(t *testing.T) {
	if (distOf(nil) != Dist{}) {
		t.Error("empty distribution")
	}
	in := []float64{9, 1, 5, 3, 7}
	d := distOf(in)
	if d.N != 5 || d.P50 != 5 || d.P90 != 9 || d.P99 != 9 || d.Max != 9 {
		t.Errorf("%+v", d)
	}
	if !reflect.DeepEqual(in, []float64{9, 1, 5, 3, 7}) {
		t.Error("distOf sorted its argument")
	}
	if got := round3(1.23456); got != 1.235 {
		t.Errorf("round3 = %v", got)
	}
	if got := distOf([]float64{1.0 / 3}).P50; got != 0.333 {
		t.Errorf("rounding: %v", got)
	}
}

func TestLog2Buckets(t *testing.T) {
	got := log2Buckets([]int64{0, 1, 1, 2, 3, 4, 1000, 1024, 1 << 30, -5})
	want := []Bucket{{0, 2}, {1, 2}, {2, 2}, {4, 1}, {512, 1}, {1024, 1}, {1 << 30, 1}}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("got %v\nwant %v", got, want)
	}
	if len(log2Buckets(nil)) != 0 {
		t.Error("no samples, no buckets")
	}
	sd := sizeDistOf([]int64{10, 20, 30})
	if sd.N != 3 || sd.Max != 30 || len(sd.Buckets) != 2 {
		t.Errorf("%+v", sd)
	}
}

func TestPathPrefix(t *testing.T) {
	for _, c := range []struct{ in, want string }{
		{"/usr/lib/x86_64-linux-gnu/libc.so.6", "/usr/lib/x86_64-linux-gnu/libc.so.6"},
		{"/usr/lib/x86_64-linux-gnu/gconv/gconv-modules", "/usr/lib/x86_64-linux-gnu/gconv"},
		{"/usr//lib/", "/usr/lib"},
		{"/", "/"},
		{"", "(empty)"},
		{"a/b/c/d/e", "a/b/c/d"},
		{"/a", "/a"},
	} {
		if got := pathPrefix(c.in, 4); got != c.want {
			t.Errorf("pathPrefix(%q) = %q, want %q", c.in, got, c.want)
		}
	}
	if got := pathPrefix("/a/b/c/d", 2); got != "/a/b" {
		t.Errorf("depth 2: %q", got)
	}
}

func TestSafeText(t *testing.T) {
	red := newRedactor([]Redaction{{From: "/home/runner", To: "$HOME"}, {From: "/home/runner/work", To: "$WORKSPACE"}, {From: "/", To: "$ROOT"}, {From: "x", To: "y"}})
	for _, c := range []struct{ in, want string }{
		{"/home/runner/.cache/x", "$HOME/.cache/x"},
		{"/home/runner/work/repo/a.go", "$WORKSPACE/repo/a.go"}, // the longer prefix wins
		{"a\nb\r\x00c\x1b[31m", "a?b??c?[31m"},
		{"evil‮gnp.exe", "evil?gnp.exe"}, // a bidi override
		{"line sep", "line?sep"},
		{"bad\xffutf8", "bad?utf8"},
		{"::set-output name=x::y", "::set-output name=x::y"}, // quoted by the Markdown layer, not here
	} {
		got, trunc := safeText(c.in, 100, red)
		if got != c.want || trunc {
			t.Errorf("safeText(%q) = %q (%v), want %q", c.in, got, trunc, c.want)
		}
	}
	long := strings.Repeat("é", 5000)
	got, trunc := safeText(long, 512, nil)
	if !trunc || utf8.RuneCountInString(got) != 512+3 || !strings.HasSuffix(got, "...") || !utf8.ValidString(got) {
		t.Errorf("a 5000-rune text: %d runes, truncated %v", utf8.RuneCountInString(got), trunc)
	}
	if got, trunc := safeText("short", 512, nil); got != "short" || trunc {
		t.Error("a short text is unchanged")
	}
	if (&redactor{}).apply("x") != "x" || (*redactor)(nil).apply("x") != "x" {
		t.Error("an empty or nil redactor changes nothing")
	}
}

// stripSpans removes every code span from s the way a Markdown reader would: a
// backtick run of length n opens a span that the next run of EXACTLY n closes.
func stripSpans(s string) string {
	var out strings.Builder
	rs := []rune(s)
	for i := 0; i < len(rs); {
		if rs[i] != '`' {
			out.WriteRune(rs[i])
			i++
			continue
		}
		n := 0
		for i+n < len(rs) && rs[i+n] == '`' {
			n++
		}
		j, closed := i+n, false
		for j < len(rs) {
			if rs[j] != '`' {
				j++
				continue
			}
			m := 0
			for j+m < len(rs) && rs[j+m] == '`' {
				m++
			}
			if m == n {
				closed = true
				j += m
				break
			}
			j += m
		}
		if !closed { // an unclosed run is literal text
			out.WriteString(strings.Repeat("`", n))
			i += n
			continue
		}
		i = j
	}
	return out.String()
}

func TestCodeSpanCannotBeEscaped(t *testing.T) {
	for _, s := range []string{
		"plain", "a`b", "`", "``", "```x```", "x`", "`x", "a``b`c```d", " lead", " both ", "|pipe|", "<script>alert(1)</script>",
		"::notice::x", "* _ # [x](y) &amp;", strings.Repeat("`", 40), "a\\`b",
	} {
		span := codeSpan(s)
		if rest := stripSpans(span); rest != "" {
			t.Errorf("codeSpan(%q) = %q leaves %q outside the span", s, span, rest)
		}
		// Surrounded by prose, the span still closes at its own end.
		if rest := stripSpans("before " + span + " after"); rest != "before  after" {
			t.Errorf("codeSpan(%q) in prose leaves %q", s, rest)
		}
	}
	if codeSpan("") != "(empty)" {
		t.Error("an empty text")
	}
}

func TestMdEscape(t *testing.T) {
	got := mdEscape("a\\b`c|d<e>f[g]h&i*j_k#l~m")
	want := "a\\\\b\\`c\\|d\\<e\\>f\\[g\\]h\\&i\\*j\\_k\\#l\\~m"
	if got != want {
		t.Errorf("got %q want %q", got, want)
	}
}

func TestFinishMarkdown(t *testing.T) {
	got := finishMarkdown("ok\n::warning::x\nbell\x07 and   and \xff\n")
	for _, l := range strings.Split(got, "\n") {
		if strings.HasPrefix(l, "::") {
			t.Errorf("a line starts with ::: %q", l)
		}
	}
	if strings.ContainsAny(got, "\x07 \xff") || !utf8.ValidString(got) {
		t.Errorf("a control character survived: %q", got)
	}
	if !strings.Contains(got, "\\::warning::x") {
		t.Errorf("the workflow command line was not defused: %q", got)
	}
}

// A directory prefix is replaced only as a whole path prefix: a short one such as
// /tmp must not corrupt /usr/lib/tmpfiles.d, /var/tmp or /tmpx.
func TestRedactorWorksOnPathBoundaries(t *testing.T) {
	red := newRedactor([]Redaction{{From: "/tmp", To: "$TMPDIR"}, {From: "/home/u", To: "$HOME"}})
	for _, c := range []struct{ in, want string }{
		{"/tmp", "$TMPDIR"},
		{"/tmp/y", "$TMPDIR/y"},
		{"a=/tmp/z b=/tmp", "a=$TMPDIR/z b=$TMPDIR"},
		{"/usr/lib/tmpfiles.d/x", "/usr/lib/tmpfiles.d/x"},
		{"/var/tmp/y", "/var/tmp/y"},
		{"/tmpx", "/tmpx"},
		{"/tmp.d/x", "/tmp.d/x"},
		{"/tmp/a /var/tmp/b /tmp/c", "$TMPDIR/a /var/tmp/b $TMPDIR/c"},
		{"/home/user/x", "/home/user/x"},
		{"/home/u/x", "$HOME/x"},
		{"/tmp/tmp", "$TMPDIR/tmp"},
		{"-L/tmp/x -I/home/u/inc", "-L$TMPDIR/x -I$HOME/inc"},
		{"cc -L/tmp/x", "cc -L$TMPDIR/x"},
		{"foo-L/tmp/x", "foo-L/tmp/x"},
		{"-LL/tmp/x", "-LL/tmp/x"},
		{"/usr/include/c++/tmp", "/usr/include/c++/tmp"},
		{"user@/tmp/x", "user@/tmp/x"},
		{"~/tmp/x", "~/tmp/x"},
		{"+/tmp/x", "+/tmp/x"},
		{"(/tmp/x) [/tmp/y] <$/tmp/z>", "($TMPDIR/x) [$TMPDIR/y] <$/tmp/z>"},
		{"a,/tmp/b:/tmp/c\"/tmp/d'/tmp/e", "a,$TMPDIR/b:$TMPDIR/c\"$TMPDIR/d'$TMPDIR/e"},
	} {
		if got := red.apply(c.in); got != c.want {
			t.Errorf("apply(%q) = %q, want %q", c.in, got, c.want)
		}
	}
}

// A language or kind that could autolink or render as an emoji is in a code span
// in a list item, never bare and never in a table cell.
func TestMarkdownLanguageAndKindAreCodeSpans(t *testing.T) {
	s := &CensusSummary{
		Languages: []LangSummary{{Language: "www.evil.example", Actions: 1}, {Language: "a:smile:b", Actions: 1}},
		Reasons:   []KindSummary{{Kind: ":smile:"}, {Kind: "www.evil.example"}},
	}
	md, _, _ := RenderCensusMarkdown(s)
	outside := stripSpans(md)
	for _, bad := range []string{"www.evil", ":smile:", "a:smile"} {
		if strings.Contains(outside, bad) {
			t.Errorf("%q occurs outside a code span", bad)
		}
	}
	for _, l := range strings.Split(md, "\n") {
		if strings.HasPrefix(l, "|") && (strings.Contains(l, "evil") || strings.Contains(l, "smile")) {
			t.Errorf("a table row holds a name: %q", l)
		}
	}
}
