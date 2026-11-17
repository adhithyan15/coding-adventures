package stracelexer

import (
	"math/rand"
	"os"
	"reflect"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// The grammar data itself
// ---------------------------------------------------------------------------

func TestGrammarsCompile(t *testing.T) {
	if c, err := Compiled(); err != nil || c == nil {
		t.Fatalf("line grammar: %v", err)
	}
	if c, err := StringCompiled(); err != nil || c == nil {
		t.Fatalf("string grammar: %v", err)
	}
}

// The embedded Go data must be exactly what the committed .tokens files say.
// There is no CI check that regenerates grammar_data.go, so this is the drift
// check: it re-parses the files and compares deeply.
func TestEmbeddedGrammarsMatchFiles(t *testing.T) {
	for _, c := range []struct {
		file string
		got  *grammartools.TokenGrammar
	}{
		{"strace.tokens", TokenGrammarData},
		{"strace-string.tokens", StringTokenGrammarData},
	} {
		src, err := os.ReadFile("../../../grammars/strace/" + c.file)
		if err != nil {
			t.Fatal(err)
		}
		want, err := grammartools.ParseTokenGrammar(string(src))
		if err != nil {
			t.Fatal(err)
		}
		if !sameData(reflect.ValueOf(want), reflect.ValueOf(c.got)) {
			t.Errorf("%s: embedded data differs from the file; regenerate with grammar-tools compile-tokens", c.file)
		}
	}
}

// sameData is reflect.DeepEqual except that a nil slice or map equals an empty
// one: the generator writes `[]string{}` where the parser leaves nil.
func sameData(a, b reflect.Value) bool {
	if a.Kind() != b.Kind() || a.Type() != b.Type() {
		return false
	}
	switch a.Kind() {
	case reflect.Ptr, reflect.Interface:
		if a.IsNil() || b.IsNil() {
			return a.IsNil() == b.IsNil()
		}
		return sameData(a.Elem(), b.Elem())
	case reflect.Slice:
		if a.Len() != b.Len() {
			return false
		}
		for i := 0; i < a.Len(); i++ {
			if !sameData(a.Index(i), b.Index(i)) {
				return false
			}
		}
		return true
	case reflect.Map:
		if a.Len() != b.Len() {
			return false
		}
		for _, k := range a.MapKeys() {
			bv := b.MapIndex(k)
			if !bv.IsValid() || !sameData(a.MapIndex(k), bv) {
				return false
			}
		}
		return true
	case reflect.Struct:
		for i := 0; i < a.NumField(); i++ {
			if !sameData(a.Field(i), b.Field(i)) {
				return false
			}
		}
		return true
	}
	return reflect.DeepEqual(a.Interface(), b.Interface())
}

// Case sensitivity is a security requirement (see strace.tokens).
func TestGrammarsAreCaseSensitive(t *testing.T) {
	for name, g := range map[string]*grammartools.TokenGrammar{"line": TokenGrammarData, "string": StringTokenGrammarData} {
		if !g.CaseSensitive || g.CaseInsensitive {
			t.Errorf("%s grammar is not case-sensitive", name)
		}
	}
}

// Every rule must be a literal or a proven-deterministic matcher.
func TestNoSlowRules(t *testing.T) {
	if s := SlowRules(); len(s) != 0 {
		t.Fatalf("rules on the regexp engine: %v", s)
	}
	c, _ := StringCompiled()
	if s := c.SlowRules(); len(s) != 0 {
		t.Fatalf("string grammar rules on the regexp engine: %v", s)
	}
}

// A token whose name contains STRING and starts with a quote loses its first
// and last byte in the lexer.
func TestNoTokenNamedString(t *testing.T) {
	for _, g := range []*grammartools.TokenGrammar{TokenGrammarData, StringTokenGrammarData} {
		all := append([]grammartools.TokenDefinition{}, g.Definitions...)
		all = append(all, g.SkipDefinitions...)
		for _, grp := range g.Groups {
			all = append(all, grp.Definitions...)
		}
		for _, d := range all {
			if strings.Contains(d.Name, "STRING") || strings.Contains(d.Alias, "STRING") {
				t.Errorf("token %s must not contain STRING", d.Name)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Token shapes
// ---------------------------------------------------------------------------

type tv struct{ typ, val string }

func lex(t *testing.T, line string) []tv {
	t.Helper()
	toks, err := TokenizeLine(line)
	if err != nil {
		t.Fatalf("%q: %v", line, err)
	}
	var out []tv
	for _, tk := range toks {
		out = append(out, tv{tk.TypeName, tk.Value})
	}
	if out[len(out)-1].typ != "EOF" {
		t.Fatalf("%q: no EOF", line)
	}
	return out[:len(out)-1]
}

func TestTokenShapes(t *testing.T) {
	cases := []struct {
		line string
		want []tv
	}{
		{`100  openat(AT_FDCWD</w>, "/e"..., O_RDONLY|O_CLOEXEC) = 3</e>`, []tv{
			{"PID", "100  "}, {"SYSCALL", "openat("}, {"ANNOTATED", "AT_FDCWD</w>"}, {"COMMA", ","},
			{"QUOTED", `"/e"...`}, {"COMMA", ","}, {"WORD", "O_RDONLY"}, {"PUNCT", "|"}, {"WORD", "O_CLOEXEC"},
			{"RPAREN", ")"}, {"PUNCT", "="}, {"ANNOTATED", "3</e>"}}},
		{`[pid 12] wait4(-1,  <unfinished ...>`, []tv{
			{"PID_TAG", "[pid 12] "}, {"SYSCALL", "wait4("}, {"WORD", "-1"}, {"COMMA", ","}, {"UNFINISHED", "<unfinished ...>"}}},
		{`7  <... wait4 resumed>) = 1`, []tv{
			{"PID", "7  "}, {"RESUMED", "<... wait4 resumed>"}, {"RPAREN", ")"}, {"PUNCT", "="}, {"WORD", "1"}}},
		{`7  <... ??? resumed>) = ?`, []tv{
			{"PID", "7  "}, {"DEAD_RESUMED", "<... ??? resumed>) = ?"}}},
		{`7  ???( <unfinished ...>`, []tv{{"PID", "7  "}, {"DEAD_OPEN", "???( <unfinished ...>"}}},
		{`7  ???()   = ?`, []tv{{"PID", "7  "}, {"DEAD_WHOLE", "???()   = ?"}}},
		{`7  +++ exited with 0 +++`, []tv{{"PID", "7  "}, {"EXITED_LINE", "+++ exited with 0 +++"}}},
		{`7  +++ killed by SIGKILL +++`, []tv{{"PID", "7  "}, {"KILLED_LINE", "+++ killed by SIGKILL +++"}}},
		{`7  +++ killed by SIGSEGV (core dumped) +++`, []tv{{"PID", "7  "}, {"KILLED_CORE_LINE", "+++ killed by SIGSEGV (core dumped) +++"}}},
		{`7  --- SIGCHLD {si_pid=1} ---`, []tv{
			{"PID", "7  "}, {"SIG_OPEN", "--- "}, {"SIG_TEXT", "SIGCHLD"}, {"SIG_SP", " "}, {"SIG_TEXT", "{si_pid=1}"},
			{"SIG_SP", " "}, {"SIG_TEXT", "---"}}},
		{`7  poll([{fd=3</x>, events=POLLIN}], 1, 0) = 1`, []tv{
			{"PID", "7  "}, {"SYSCALL", "poll("}, {"LBRACKET", "["}, {"LBRACE", "{"}, {"ASSIGN", "fd="}, {"ANNOTATED", "3</x>"},
			{"COMMA", ","}, {"ASSIGN", "events="}, {"WORD", "POLLIN"}, {"RBRACE", "}"}, {"RBRACKET", "]"}, {"COMMA", ","},
			{"WORD", "1"}, {"COMMA", ","}, {"WORD", "0"}, {"RPAREN", ")"}, {"PUNCT", "="}, {"WORD", "1"}}},
		{`7  f(WIFEXITED(s) && x, htons(80)) = 0 /* 3 in strace's PID NS */`, []tv{
			{"PID", "7  "}, {"SYSCALL", "f("}, {"FUNC", "WIFEXITED("}, {"WORD", "s"}, {"RPAREN", ")"}, {"PUNCT", "&&"},
			{"WORD", "x"}, {"COMMA", ","}, {"SYSCALL", "htons("}, {"WORD", "80"}, {"RPAREN", ")"}, {"RPAREN", ")"},
			{"PUNCT", "="}, {"WORD", "0"}, {"PUNCT", "/*"}, {"WORD", "3"}, {"WORD", "in"}, {"WORD", "strace"},
			{"PUNCT", "'"}, {"WORD", "s"}, {"WORD", "PID"}, {"WORD", "NS"}, {"PUNCT", "*/"}}},
		// An escaped quote and an escaped backslash do not end a string; the
		// value is untouched (no quote stripping).
		{`7  f("a\"b\\", "c")`, []tv{
			{"PID", "7  "}, {"SYSCALL", "f("}, {"QUOTED", `"a\"b\\"`}, {"COMMA", ","}, {"QUOTED", `"c"`}, {"RPAREN", ")"}}},
		// A non-ASCII byte outside a string is punctuation, not an error.
		{"7  f(\xc3\xa9)", []tv{{"PID", "7  "}, {"SYSCALL", "f("}, {"PUNCT", "\xc3\xa9"}, {"RPAREN", ")"}}},
		{`7  f(a=b)`, []tv{{"PID", "7  "}, {"SYSCALL", "f("}, {"ASSIGN", "a="}, {"WORD", "b"}, {"RPAREN", ")"}}},
		// A space before the "(" means it is not a call name.
		{`7  f (1)`, []tv{{"PID", "7  "}, {"WORD", "f"}, {"LPAREN", "("}, {"WORD", "1"}, {"RPAREN", ")"}}},
		// An uppercase name is a FUNC, not a SYSCALL.
		{`7  Foo(1)`, []tv{{"PID", "7  "}, {"FUNC", "Foo("}, {"WORD", "1"}, {"RPAREN", ")"}}},
		// 65 characters is too long for a syscall name.
		{`7  ` + strings.Repeat("a", 65) + `(`, []tv{{"PID", "7  "}, {"FUNC", strings.Repeat("a", 65) + "("}}},
		{`7  [ ] { }`, []tv{{"PID", "7  "}, {"LBRACKET", "["}, {"RBRACKET", "]"}, {"LBRACE", "{"}, {"RBRACE", "}"}}},
	}
	for _, c := range cases {
		got := lex(t, c.line)
		if !reflect.DeepEqual(got, c.want) {
			t.Errorf("%q\n got  %v\n want %v", c.line, got, c.want)
		}
	}
}

// Pids: 1 to 9 digits, no leading zero, never 0.
func TestPidPrefix(t *testing.T) {
	good := []string{"1  x(", "123456789 x(", "[pid 1] x(", "[pid 123456789]x(", "9 x("}
	for _, l := range good {
		if first := lex(t, l)[0].typ; first != "PID" && first != "PID_TAG" {
			t.Errorf("%q: first token %s", l, first)
		}
	}
	bad := []string{"0  x(", "01  x(", "1234567890  x(", "[pid 0] x(", "[pid 007] x(", "[pid  1] x(", "[pid x] x(", "1x(", "[pid 1 x(", "1", ""}
	for _, l := range bad {
		toks := lex(t, l)
		if len(toks) > 0 && toks[0].typ != "ERR_PID" {
			t.Errorf("%q: first token %v, want ERR_PID", l, toks[0])
		}
	}
	// The pid prefix is only special at the start of the line.
	for _, tk := range lex(t, "1  f(41 vars)")[1:] {
		if tk.typ == "PID" || tk.typ == "PID_TAG" {
			t.Errorf("a number in the arguments lexed as a pid: %v", tk)
		}
	}
}

// Exit lines are exact: one space inside, 1..9 digits, at least one space
// before the closing +++.
func TestExitLineExactness(t *testing.T) {
	notExit := []string{
		"+++exited with 0 +++", "+++ exited  with 0 +++", "+++ exited with 0+++", "+++ exited with 1234567890 +++",
		"+++ exited with x +++", "+++ exited with +5 +++", "+++ killed by sigkill +++", "+++ killed by SIGKILL (core dumped)+++",
		"+++ killed by SIGSEGV  (core dumped) +++", "+++ exited with 0", "+++ vanished +++",
	}
	for _, body := range notExit {
		for _, tk := range lex(t, "1  "+body) {
			if strings.HasSuffix(tk.typ, "_LINE") {
				t.Errorf("%q lexed as %s", body, tk.typ)
			}
		}
	}
	// Extra spaces around the inner text are allowed (the old parser trims).
	if tk := lex(t, "1  +++   exited with 5    +++")[1]; tk.typ != "EXITED_LINE" {
		t.Errorf("padded exit line: %v", tk)
	}
}

// A signal line is lexed in a mode where spaces are tokens and the rest is
// SIG_TEXT; the grammar finds the closing " ---".
func TestSignalMode(t *testing.T) {
	got := lex(t, "1  --- SIGCHLD {} ---")
	want := []tv{{"PID", "1  "}, {"SIG_OPEN", "--- "}, {"SIG_TEXT", "SIGCHLD"}, {"SIG_SP", " "}, {"SIG_TEXT", "{}"}, {"SIG_SP", " "}, {"SIG_TEXT", "---"}}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("got %v", got)
	}
	// Spaces stay tokens to the end of the line (no skipping, so trailing
	// spaces are visible).
	if got := lex(t, "1  --- a ---  "); got[len(got)-1] != (tv{"SIG_SP", "  "}) {
		t.Errorf("trailing spaces: %v", got)
	}
	// "--- ---": the one space after --- is part of the opener, so there is no
	// space token before the closing text.
	if got := lex(t, "1  --- ---"); !reflect.DeepEqual(got[1:], []tv{{"SIG_OPEN", "--- "}, {"SIG_TEXT", "---"}}) {
		t.Errorf("--- --- : %v", got)
	}
	// "---  ---" (two spaces) has the space token.
	if got := lex(t, "1  ---  ---"); !reflect.DeepEqual(got[1:], []tv{{"SIG_OPEN", "--- "}, {"SIG_SP", " "}, {"SIG_TEXT", "---"}}) {
		t.Errorf("---  --- : %v", got)
	}
	// Whatever is in the text, an unterminated quote or a lone bracket, is text.
	for _, tk := range lex(t, `1  --- a="x [ ( ---`)[2:] {
		if !strings.HasPrefix(tk.typ, "SIG_") {
			t.Errorf("not signal text: %v", tk)
		}
	}
	// A control character is still an error token.
	found := false
	for _, tk := range lex(t, "1  --- a\tb ---") {
		found = found || tk.typ == "ERR_CTRL"
	}
	if !found {
		t.Error("tab in a signal line is not ERR_CTRL")
	}
	// "--- " only opens a signal line at the start of the body: inside an
	// argument list it is plain text again.
	for _, tk := range lex(t, "1  f(--- x)") {
		if strings.HasPrefix(tk.typ, "SIG_") {
			t.Errorf("signal token inside an argument list: %v", tk)
		}
	}
	// The tokens that only begin a body are plain text elsewhere.
	for _, line := range []string{"1  f(???(, +++ exited with 0 +++)", "1  f(x) = <... g resumed>"} {
		for _, tk := range lex(t, line) {
			switch tk.typ {
			case "DEAD_OPEN", "DEAD_WHOLE", "DEAD_RESUMED", "EXITED_LINE", "RESUMED", "SIG_OPEN":
				t.Errorf("%q: body-start token %s in the middle of a line", line, tk.typ)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// ERR tokens
// ---------------------------------------------------------------------------

func TestErrTokens(t *testing.T) {
	cases := []struct {
		line string
		err  string
	}{
		{`1  f("abc`, "ERR_QUOTE"},
		{`1  f("a` + "\x01" + `b")`, "ERR_QUOTE"},
		{`1  f(3<abc`, "ERR_ANNOT_OPEN"},
		{`1  f(3<a<b>)`, "ERR_ANNOT_BAD"},
		{"1  f(3<a\x01b>)", "ERR_ANNOT_BAD"},
		{`1  f(1<<2)`, "ERR_ANNOT_OPEN"},
		{`1  f(3 <x>)`, "ERR_LT"},
		{`1  f("x"<x>)`, "ERR_LT"},
		{`1  <... stat`, "ERR_RESUME"},
		{`1  <...x resumed>`, "ERR_RESUME"},
		{`1  <... st at resumed>`, "ERR_RESUME"},
		{"1  f(\t)", "ERR_CTRL"},
		{"1  f(\x00)", "ERR_CTRL"},
		{"1  f(\x7f)", "ERR_CTRL"},
		{"1  f(\r)", "ERR_CTRL"},
		{`0  f()`, "ERR_PID"},
	}
	for _, c := range cases {
		toks, err := TokenizeLine(c.line)
		if err != nil {
			t.Errorf("%q: %v", c.line, err)
			continue
		}
		e, i, ok := FirstError(toks)
		if !ok || e.TypeName != c.err {
			t.Errorf("%q: first error %v at %d (ok=%v), want %s", c.line, e.TypeName, i, ok, c.err)
		}
	}
	if _, _, ok := FirstError(toksOf(t, `1  f(a)`)); ok {
		t.Error("clean line reported an error")
	}
}

func toksOf(t *testing.T, l string) []lexer.Token {
	t.Helper()
	toks, err := TokenizeLine(l)
	if err != nil {
		t.Fatal(err)
	}
	return toks
}

func TestNewlineRefused(t *testing.T) {
	if _, err := TokenizeLine("1  f()\n"); err != ErrNewline {
		t.Errorf("got %v", err)
	}
	if _, err := TokenizeLine("1  f(\n)"); err != ErrNewline {
		t.Errorf("got %v", err)
	}
}

// ERR_ANNOT_OPEN is "ambiguous" when the old scanner would have hit a bad
// character before running off the end, and "unterminated" otherwise.
func TestAmbiguous(t *testing.T) {
	cases := []struct {
		line string
		amb  bool
	}{
		{`1  f(3<abc`, false},
		{`1  f(3<a b c`, false},
		{`1  f(3<a<b`, true},
		{`1  f(3<a\qb`, true},
		{`1  f(3<a\`, true},
		{`1  f(3<a\n`, false},
		{`1  f(3<a<b>)`, true},
		{`1  f("abc`, false},
		{`1  f(3 <x>)`, false},
	}
	for _, c := range cases {
		e, _, ok := FirstError(toksOf(t, c.line))
		if !ok {
			t.Fatalf("%q: no error token", c.line)
		}
		if got := Ambiguous(e); got != c.amb {
			t.Errorf("%q (%s): Ambiguous = %v, want %v", c.line, e.TypeName, got, c.amb)
		}
	}
}

func TestBodyHelpers(t *testing.T) {
	var ann, q lexer.Token
	for _, tk := range toksOf(t, `1  f(AT_FDCWD</a b>, "x\"y"..., "z")`) {
		switch tk.TypeName {
		case "ANNOTATED":
			ann = tk
		case "QUOTED":
			if q.Value == "" {
				q = tk
			}
		}
	}
	if w, b := AnnotationBody(ann); w != "AT_FDCWD" || b != "/a b" {
		t.Errorf("annotation: %q %q", w, b)
	}
	if b, trunc := QuotedBody(q); b != `x\"y` || !trunc {
		t.Errorf("quoted: %q %v", b, trunc)
	}
	if b, trunc := QuotedBody(lexer.Token{Value: `"ab"`}); b != "ab" || trunc {
		t.Errorf("quoted plain: %q %v", b, trunc)
	}
}

// ---------------------------------------------------------------------------
// Escape decoding
// ---------------------------------------------------------------------------

func TestDecodeString(t *testing.T) {
	ok := []struct{ in, out string }{
		{``, ``},
		{`/etc/passwd`, `/etc/passwd`},
		{`a\tb\"c\\d`, "a\tb\"c\\d"},
		{`\n\r\a\b\f\v`, "\n\r\a\b\f\v"},
		{`\303\251`, "\xc3\xa9"},
		{`\0`, "\x00"},
		{`\12`, "\n"},
		{`\1234`, "S4"}, // three octal digits, then a plain 4
		{`\377`, "\xff"},
		{`\x41\x4a\x4A`, "AJJ"},
		{`a<b>c`, "a<b>c"},
		{"caf\xc3\xa9", "caf\xc3\xa9"},
		{"a b", "a b"},
	}
	for _, c := range ok {
		got, err := DecodeString(c.in)
		if err != nil || string(got) != c.out {
			t.Errorf("DecodeString(%q) = %q, %v; want %q", c.in, got, err, c.out)
		}
	}
	bad := []string{`\400`, `\777`, `\x4`, `\xzz`, `\x`, `\q`, `\`, `a\`, `\<`, `\>`, "a\x01", "a\x7f", "a\tb", `a"b`}
	for _, in := range bad {
		if got, err := DecodeString(in); err == nil {
			t.Errorf("DecodeString(%q) = %q, want an error", in, got)
		} else if _, isDE := err.(*DecodeError); !isDE || err.Error() == "" {
			t.Errorf("DecodeString(%q): error %T %v", in, err, err)
		}
	}
}

func TestDecodeAnnotation(t *testing.T) {
	ok := []struct{ in, out string }{
		{`/etc/passwd`, `/etc/passwd`},
		{`a\<b\>c`, `a<b>c`},
		{`a"b`, `a"b`}, // a raw quote is ordinary in an annotation
		{`\303\251`, "\xc3\xa9"},
		{`socket:[1]`, `socket:[1]`},
	}
	for _, c := range ok {
		got, err := DecodeAnnotation(c.in)
		if err != nil || string(got) != c.out {
			t.Errorf("DecodeAnnotation(%q) = %q, %v; want %q", c.in, got, err, c.out)
		}
	}
	for _, in := range []string{`a<b`, `a>b`, `\400`, `\q`, `\`, "a\x01"} {
		if got, err := DecodeAnnotation(in); err == nil {
			t.Errorf("DecodeAnnotation(%q) = %q, want an error", in, got)
		}
	}
}

// ---------------------------------------------------------------------------
// Corpus, totality and fuzzing
// ---------------------------------------------------------------------------

func corpus(t testing.TB) []string {
	b, err := os.ReadFile("testdata/corpus.txt")
	if err != nil {
		t.Fatal(err)
	}
	return strings.Split(strings.TrimSuffix(string(b), "\n"), "\n")
}

// checkTokens verifies what every consumer relies on: the tokens, with the
// skipped spaces, tile the line.  Each token's Value is exactly line at its
// Column, columns increase, and only spaces lie between tokens.
func checkTokens(t testing.TB, line string, toks []lexer.Token) {
	t.Helper()
	if len(toks) == 0 || toks[len(toks)-1].TypeName != "EOF" {
		t.Fatalf("%q: no final EOF", line)
	}
	pos := 0
	for i, tk := range toks[:len(toks)-1] {
		if tk.TypeName == "EOF" {
			t.Fatalf("%q: EOF before the end (token %d)", line, i)
		}
		if tk.Value == "" {
			t.Fatalf("%q: empty token %s", line, tk.TypeName)
		}
		s := tk.Column - 1
		if s < pos || s+len(tk.Value) > len(line) || line[s:s+len(tk.Value)] != tk.Value {
			t.Fatalf("%q: token %s %q at column %d does not match the line", line, tk.TypeName, tk.Value, tk.Column)
		}
		if strings.Trim(line[pos:s], " ") != "" {
			t.Fatalf("%q: non-space text %q skipped before %s", line, line[pos:s], tk.TypeName)
		}
		pos = s + len(tk.Value)
	}
	if strings.Trim(line[pos:], " ") != "" {
		t.Fatalf("%q: unlexed tail %q", line, line[pos:])
	}
}

func TestCorpusLexes(t *testing.T) {
	lines := corpus(t)
	if len(lines) < 300 {
		t.Fatalf("corpus has only %d lines", len(lines))
	}
	for _, l := range lines {
		toks, err := TokenizeLine(l)
		if err != nil {
			t.Fatalf("%q: %v", l, err)
		}
		checkTokens(t, l, toks)
	}
}

// The grammar is total: random printable (and not so printable) input, built
// from the characters that matter to strace syntax, always lexes.
func TestRandomInputIsTotal(t *testing.T) {
	alpha := []string{"<", ">", "\"", "\\", "(", ")", "[", "]", "{", "}", ",", " ", "  ", "=", "|", "+++", "---", "...", "unfinished",
		"resumed", "<... ", " resumed>", "3", "a", "_", "-", "x", "\\x", "\\0", "\t", "\r", "\x00", "\x7f", "\xc3", "\xff", "é", "???", "SIG", "1  ", "[pid 5] ", "/*", "*/", "'", "?"}
	r := rand.New(rand.NewSource(1))
	for n := 0; n < 20000; n++ {
		var sb strings.Builder
		for k := r.Intn(30); k >= 0; k-- {
			sb.WriteString(alpha[r.Intn(len(alpha))])
		}
		l := sb.String()
		toks, err := TokenizeLine(l)
		if err != nil {
			t.Fatalf("%q: %v", l, err)
		}
		checkTokens(t, l, toks)
	}
}

func FuzzTokenizeLine(f *testing.F) {
	for _, l := range corpusOrNil() {
		f.Add(l)
	}
	f.Fuzz(func(t *testing.T, line string) {
		toks, err := TokenizeLine(line)
		if strings.Contains(line, "\n") {
			if err != ErrNewline {
				t.Fatalf("newline: %v", err)
			}
			return
		}
		if err != nil {
			t.Fatalf("%q: %v", line, err)
		}
		checkTokens(t, line, toks)
	})
}

func corpusOrNil() []string {
	b, err := os.ReadFile("testdata/corpus.txt")
	if err != nil {
		return nil
	}
	return strings.Split(strings.TrimSuffix(string(b), "\n"), "\n")
}

func FuzzDecode(f *testing.F) {
	for _, s := range []string{``, `a\tb`, `\303\251`, `\x41`, `\400`, `\`, `a<b`, `\<`} {
		f.Add(s)
	}
	f.Fuzz(func(t *testing.T, body string) {
		a, aerr := DecodeAnnotation(body)
		s, serr := DecodeString(body)
		// Whatever decodes is no longer than its source.
		if aerr == nil && len(a) > len(body) {
			t.Fatalf("annotation grew: %q -> %q", body, a)
		}
		if serr == nil && len(s) > len(body) {
			t.Fatalf("string grew: %q -> %q", body, s)
		}
	})
}

// Whatever the lexer delimits as one ANNOTATED token or QUOTED token never
// contains a newline or control character.
func TestQuotedAnnotatedNeverContainControls(t *testing.T) {
	r := rand.New(rand.NewSource(2))
	alpha := []string{"\"", "<", ">", "\\", "a", "3", "\x01", "\x7f", " ", "\t", "..."}
	for n := 0; n < 5000; n++ {
		var sb strings.Builder
		for k := r.Intn(12); k >= 0; k-- {
			sb.WriteString(alpha[r.Intn(len(alpha))])
		}
		for _, tk := range toksOf(t, "1  f("+sb.String()) {
			if tk.TypeName == "QUOTED" || tk.TypeName == "ANNOTATED" {
				for i := 0; i < len(tk.Value); i++ {
					if c := tk.Value[i]; c < 0x20 || c == 0x7f {
						t.Fatalf("%s %q contains a control character", tk.TypeName, tk.Value)
					}
				}
			}
		}
	}
}
