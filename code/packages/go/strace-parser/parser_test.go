package straceparser

import (
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	stracelexer "github.com/adhithyan15/coding-adventures/code/packages/go/strace-lexer"
)

// ---------------------------------------------------------------------------
// The grammar data itself
// ---------------------------------------------------------------------------

func TestGrammarCompiles(t *testing.T) {
	if c, err := Compiled(); err != nil || c == nil {
		t.Fatalf("grammar did not compile: %v", err)
	}
}

// The embedded Go data must be exactly what strace.grammar says (there is no CI
// check that regenerates grammar_data.go, so this is the drift check).
func TestEmbeddedGrammarMatchesFile(t *testing.T) {
	src, err := os.ReadFile("../../../grammars/strace/strace.grammar")
	if err != nil {
		t.Fatal(err)
	}
	want, err := grammartools.ParseParserGrammar(string(src))
	if err != nil {
		t.Fatal(err)
	}
	if !sameData(reflect.ValueOf(want), reflect.ValueOf(ParserGrammarData)) {
		t.Fatal("embedded grammar differs from strace.grammar; regenerate with grammar-tools compile-grammar")
	}
}

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

// What this enforces: every terminal in strace.grammar is a token reference to
// a token that exists in strace.tokens, EXCEPT exactly one string literal,
// "---" (in signal_close; a literal matches a token by its text whatever its
// type, which is safe there because a signal line only has SIG_SP and SIG_TEXT
// tokens).  No ERR_* token is referenced, so the grammar can never accept one.
func TestGrammarTerminals(t *testing.T) {
	known := map[string]bool{}
	add := func(defs []grammartools.TokenDefinition) {
		for _, d := range defs {
			known[d.Name] = true
		}
	}
	add(stracelexer.TokenGrammarData.Definitions)
	for _, g := range stracelexer.TokenGrammarData.Groups {
		add(g.Definitions)
	}
	known["EOF"] = true
	literals := 0
	var walk func(e grammartools.GrammarElement)
	walk = func(e grammartools.GrammarElement) {
		switch v := e.(type) {
		case grammartools.Literal:
			// The one justified literal: the text of the last token of a signal
			// line (see signal_close in strace.grammar).
			if v.Value != "---" {
				t.Errorf("literal %q in the grammar", v.Value)
			}
			literals++
		case grammartools.RuleReference:
			if v.IsToken && !known[v.Name] {
				t.Errorf("token %s is not defined in strace.tokens", v.Name)
			}
			if strings.HasPrefix(v.Name, "ERR_") {
				t.Errorf("the grammar must not accept %s", v.Name)
			}
		case grammartools.Sequence:
			for _, x := range v.Elements {
				walk(x)
			}
		case grammartools.Alternation:
			for _, x := range v.Choices {
				walk(x)
			}
		case grammartools.Repetition:
			walk(v.Element)
		case grammartools.Optional:
			walk(v.Element)
		case grammartools.Group:
			walk(v.Element)
		case grammartools.OneOrMoreRepetition:
			walk(v.Element)
		case grammartools.PositiveLookahead:
			walk(v.Element)
		case grammartools.NegativeLookahead:
			walk(v.Element)
		default:
			t.Errorf("unexpected element %T", e)
		}
	}
	for _, r := range ParserGrammarData.Rules {
		walk(r.Body)
	}
	if literals != 1 {
		t.Errorf("%d literals, want exactly 1", literals)
	}
	if ParserGrammarData.Rules[0].Name != StartRule {
		t.Errorf("first rule is %s, want %s", ParserGrammarData.Rules[0].Name, StartRule)
	}
}

func TestLexerIsFast(t *testing.T) {
	if s := stracelexer.SlowRules(); len(s) != 0 {
		t.Fatalf("slow lexer rules: %v", s)
	}
}

func TestDefaultOptions(t *testing.T) {
	o := DefaultOptions()
	if o.MaxDepth != DefaultMaxDepth || o.MaxTokens != 65536 || o.StartRule != "line" {
		t.Fatalf("%+v", o)
	}
	if o.MaxMemoSlots != len(ParserGrammarData.Rules)*(65536+1) {
		t.Fatalf("MaxMemoSlots %d", o.MaxMemoSlots)
	}
}

// ---------------------------------------------------------------------------
// Accepted shapes
// ---------------------------------------------------------------------------

func TestAcceptedShapes(t *testing.T) {
	cases := []struct {
		line  string
		kind  string
		args  []string // nil: not checked
		after string   // text after ")", only when hasAfter
		hasAf bool
	}{
		// calls
		{`100  openat(AT_FDCWD</w>, "/etc/passwd", O_RDONLY|O_CLOEXEC) = 3</etc/passwd>`, "call_line",
			[]string{`AT_FDCWD</w>`, `"/etc/passwd"`, `O_RDONLY|O_CLOEXEC`}, ` = 3</etc/passwd>`, true},
		{`1  openat(AT_FDCWD, "/nope", O_RDONLY) = -1 ENOENT (No such file or directory)`, "call_line",
			[]string{`AT_FDCWD`, `"/nope"`, `O_RDONLY`}, ` = -1 ENOENT (No such file or directory)`, true},
		{`[pid 5] getpid()                         = 5`, "call_line", []string{}, `                         = 5`, true},
		{`1  f( )`, "call_line", []string{}, ``, true},
		{`1  newfstatat(AT_FDCWD</r>, "go.mod", {st_mode=S_IFREG|0644, st_size=40, ...}, 0) = 0`, "call_line",
			[]string{`AT_FDCWD</r>`, `"go.mod"`, `{st_mode=S_IFREG|0644, st_size=40, ...}`, `0`}, ` = 0`, true},
		{`1  execve("/usr/bin/sh", ["sh", "-c", "make"], 0x7ffd1c2e3a40 /* 41 vars */) = 0`, "call_line",
			[]string{`"/usr/bin/sh"`, `["sh", "-c", "make"]`, `0x7ffd1c2e3a40 /* 41 vars */`}, ` = 0`, true},
		{`1  connect(3<socket:[1]>, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("93.184.216.34")}, 16) = -1 EINPROGRESS (Operation now in progress)`, "call_line",
			[]string{`3<socket:[1]>`, `{sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("93.184.216.34")}`, `16`}, ` = -1 EINPROGRESS (Operation now in progress)`, true},
		{`1  connect(5, {sa_family=AF_UNIX, sun_path=@"abstract name"}, 15) = 0`, "call_line", nil, ``, false},
		{`1  wait4(-1, [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 4`, "call_line",
			[]string{`-1`, `[{WIFEXITED(s) && WEXITSTATUS(s) == 0}]`, `0`, `NULL`}, ` = 4`, true},
		{`1  clone(child_stack=NULL, flags=CLONE_CHILD_CLEARTID|SIGCHLD, child_tidptr=0x7f) = 2 /* 5030 in strace's PID NS */`, "call_line", nil, ``, false},
		{`1  read(3</x>, "a\"b\\c\303\251"..., 4096) = 4096`, "call_line", []string{`3</x>`, `"a\"b\\c\303\251"...`, `4096`}, ` = 4096`, true},
		{`1  openat(AT_FDCWD</a\<b\>>, "x", 0) = 3</a\<b\>>`, "call_line", []string{`AT_FDCWD</a\<b\>>`, `"x"`, `0`}, ` = 3</a\<b\>>`, true},
		{`1  stat("a", 0x1) = ?`, "call_line", nil, ``, false},
		{`1  exit_group(0) = ?`, "call_line", []string{`0`}, ` = ?`, true},
		{`1  f(a,"x","y")=0`, "call_line", []string{`a`, `"x"`, `"y"`}, `=0`, true},
		// interrupted
		{`1  wait4(-1,  <unfinished ...>`, "call_line", []string{`-1`}, ``, true},
		{`1  openat(AT_FDCWD</w>, "main.o", O_WRONLY|O_CREAT|O_TRUNC, 0666 <unfinished ...>`, "call_line",
			[]string{`AT_FDCWD</w>`, `"main.o"`, `O_WRONLY|O_CREAT|O_TRUNC`, `0666`}, ``, true},
		{`1  vfork( <unfinished ...>`, "call_line", []string{}, ``, true},
		{`1  f(a,<unfinished ...>`, "call_line", []string{`a`}, ``, true},
		{`1  stat("/tmp/x"<unfinished ...>`, "call_line", []string{`"/tmp/x"`}, ``, true},
		{`1  clone(child_stack=0x7f, flags=CLONE_VM|CLONE_THREAD, parent_tid=[201], tls=0x7f <unfinished ...>`, "call_line", nil, ``, false},
		// resumed
		{`1  <... wait4 resumed>[{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 101`, "resumed_line", nil, ` = 101`, true},
		{`1  <... openat resumed>)            = 6</work/main.o>`, "resumed_line", nil, `            = 6</work/main.o>`, true},
		{`1  <... clone resumed>)                    = 201`, "resumed_line", nil, `                    = 201`, true},
		{`1  <... openat resumed>, 0666) = 3`, "resumed_line", nil, ` = 3`, true},
		// The dying-call shapes (KILN04 D35): the grammar is loose, the consumer
		// pins the spelling.
		{`2  <... tgkill resumed>) = ? <unavailable>`, "resumed_line", nil, ` = ? <unavailable>`, true},
		{`2  <... clone resumed> <unfinished ...>) = ?`, "resumed_line", nil, ` = ?`, true},
		{`2  tgkill(1, 1) = ? <unavailable>`, "call_line", []string{`1`, `1`}, ` = ? <unavailable>`, true},
		// exit and signal lines
		{`1  +++ exited with 0 +++`, "exit_line", nil, ``, true},
		{`1  +++ killed by SIGKILL +++`, "exit_line", nil, ``, true},
		{`1  +++ killed by SIGSEGV (core dumped) +++`, "exit_line", nil, ``, true},
		{`1  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=102} ---`, "signal_line", nil, ``, true},
		{`1  --- SIGSEGV {si_signo=SIGSEGV, si_code=SEGV_MAPERR, si_addr=NULL} ---`, "signal_line", nil, ``, true},
		{`1  --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=101, si_uid=1000, si_status=0, si_utime=0, si_stime=0} ---`, "signal_line", nil, ``, true},
		// D18 killed-at-entry
		{`3486  ???( <unfinished ...>`, "dead_line", nil, ``, true},
		{`3486  <... ??? resumed>)          = ?`, "dead_resumed_line", nil, ``, false},
		{`3486  ???()                       = ?`, "dead_line", nil, ``, false},
	}
	for _, c := range cases {
		ast, toks, err := ParseLineTokens(c.line, DefaultOptions())
		if err != nil {
			t.Errorf("%q: %v", c.line, err)
			continue
		}
		if got := BodyKind(ast); got != c.kind {
			t.Errorf("%q: body %q, want %q", c.line, got, c.kind)
		}
		if toks[len(toks)-1].TypeName != "EOF" {
			t.Errorf("%q: no EOF", c.line)
		}
		if c.args != nil {
			if got := RawArgs(c.line, ast); !reflect.DeepEqual(got, c.args) && !(len(got) == 0 && len(c.args) == 0) {
				t.Errorf("%q:\n args %q\n want %q", c.line, got, c.args)
			}
		}
		if c.hasAf {
			if got := TextAfterClose(c.line, ast); got != c.after {
				t.Errorf("%q: after %q, want %q", c.line, got, c.after)
			}
		}
		// The two-argument form is what ParseLine returns.
		if a2, err := ParseLine(c.line, Options{}); err != nil || a2 == nil {
			t.Errorf("ParseLine(%q): %v", c.line, err)
		}
	}
}

func TestBodyHelpersOnOtherInputs(t *testing.T) {
	if Body(nil) != nil || BodyKind(nil) != "" || RawArgs("", nil) != nil || TextAfterClose("", nil) != "" {
		t.Error("helpers on a nil tree")
	}
	other := &parser.ASTNode{RuleName: "x"}
	if Body(other) != nil {
		t.Error("Body of a non-line node")
	}
	ast, _ := ParseLine(`1  +++ exited with 0 +++`, Options{})
	if RawArgs(`1  +++ exited with 0 +++`, ast) != nil || TextAfterClose(`1  +++ exited with 0 +++`, ast) != "" {
		t.Error("args or tail of an exit line")
	}
	empty := &parser.ASTNode{RuleName: StartRule}
	if Body(empty) != nil {
		t.Error("Body of a line with no children")
	}
	ast, _ = ParseLine(`1  wait4(-1, <unfinished ...>`, Options{})
	if TextAfterClose(`1  wait4(-1, <unfinished ...>`, ast) != "" {
		t.Error("an interrupted call has no text after a close")
	}
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

func TestRefusals(t *testing.T) {
	type r struct {
		line string
		kind Kind
		amb  bool
	}
	cases := []r{
		// ERR tokens
		{`1  f("abc`, KindErrToken, false},
		{"1  f(\"a\x01b\")", KindErrToken, false},
		{`1  f(3<abc`, KindErrToken, false},
		{`1  f(3<a<b>)`, KindErrToken, true},
		{`1  f(3<a<b`, KindErrToken, true},
		{`1  f(3 <x>)`, KindErrToken, false},
		{`1  f("x"<x>)`, KindErrToken, false},
		{`1  <... stat`, KindErrToken, false},
		{"1  f(\t)", KindErrToken, false},
		{"1  f()\r", KindErrToken, false},
		{`0  f()`, KindErrToken, false},
		{`007  f()`, KindErrToken, false},
		{`f()`, KindErrToken, false},
		// annotations that lex but do not decode
		{`1  f(3<a\qb>)`, KindBadAnnotation, true},
		{`1  f(3<\400>)`, KindBadAnnotation, true},
		{`1  f(3<a\x4>)`, KindBadAnnotation, true},
		{`1  f(3<a\>)`, KindErrToken, false}, // the "\>" is an escape, so the body is unterminated
		{`1  f(a) = 3<\q>`, KindBadAnnotation, true},
		// the line itself
		{` 1  f()`, KindSpace, false},
		{`1  f() `, KindSpace, false},
		{"1  f()\n", KindNewline, false},
		{"1  f(\n)", KindNewline, false},
		// syntax
		{``, KindSyntax, false},
		{`1`, KindErrToken, false},
		{`1  `, KindSpace, false},
		{`1  f(`, KindSyntax, false},
		{`1  f(a`, KindSyntax, false},
		{`1  f(a,,b)`, KindSyntax, false},
		{`1  f(a, )`, KindSyntax, false},
		{`1  f(, a)`, KindSyntax, false},
		{`1  f(a b])`, KindSyntax, false},
		{`1  f([a)]`, KindSyntax, false},
		{`1  f({a])`, KindSyntax, false},
		{`1  f((a])`, KindSyntax, false},
		{`1  f(a)) = 0`, KindSyntax, false}, // the call closed at the first ")", then kept by the free-text tail
		{`1  f (a)`, KindSyntax, false},
		{`1  Foo(a)`, KindSyntax, false},
		{`1  ` + strings.Repeat("a", 65) + `()`, KindSyntax, false},
		{`1  f(a <unfinished ...> x`, KindSyntax, false},
		{`1  f(a) <unfinished ...>`, KindSyntax, false}, // the tail has no marker (TestMarkerAfterCloseRefused)
		{`1  f([a <unfinished ...>`, KindSyntax, false},
		{`1  f(a, [b, <unfinished ...>`, KindSyntax, false},
		{`1  <... stat resumed>, <unfinished ...>`, KindSyntax, false},
		{`1  <... stat resumed>[`, KindSyntax, false},
		{`1  <... stat resumed>`, KindSyntax, false},
		{`1  +++ exited with 0`, KindSyntax, false},
		{`1  +++ exited with 0 +++ x`, KindSyntax, false},
		{`1  +++ exited with 0 +++ +++`, KindSyntax, false},
		{`1  +++exited with 0 +++`, KindSyntax, false},
		{`1  --- SIGCHLD`, KindSyntax, false},
		{`1  --- SIGCHLD {}---`, KindSyntax, false},
		{`1  --- --- x`, KindSyntax, false},
		{`1  --- SIGCHLD {} --- x`, KindSyntax, false},
		{`1  ???(0x1) = ?`, KindSyntax, false},
		{`1  ???() = 0`, KindSyntax, false},
		{`1  ???()`, KindSyntax, false},
		{`1  ???()= ?`, KindSyntax, false},
		{`1  ???() = ? x`, KindSyntax, false},
		{`1  ???(<unfinished ...>`, KindSyntax, false},
		{`1  ???(  <unfinished ...>`, KindSyntax, false},
		{`2  <... ??? resumed>) = 0`, KindErrToken, false},
		{`2  <... ??? resumed>)`, KindErrToken, false},
		{`2  <... ??? resumed>) = ? x`, KindSyntax, false},
		{`[pid +5] f() = 0`, KindErrToken, false},
		{`1  ???`, KindSyntax, false},
		{`1  ?(`, KindSyntax, false},
		{`1  (a)`, KindSyntax, false},
		{`1  ) f(`, KindSyntax, false},
	}
	for _, c := range cases {
		_, _, err := ParseLineTokens(c.line, DefaultOptions())
		if c.line == `1  f(a)) = 0` {
			// The tail is free text, so a stray ")" or a marker after the
			// close is NOT a syntax error: the consumer's result parser
			// refuses it with its own reason.
			if err != nil {
				t.Errorf("%q: %v", c.line, err)
			}
			continue
		}
		var le *LineError
		if !errors.As(err, &le) {
			t.Errorf("%q: error %v is not a LineError", c.line, err)
			continue
		}
		if le.Kind != c.kind || le.Ambiguous() != c.amb {
			t.Errorf("%q: kind %d ambiguous %v (%v), want kind %d ambiguous %v", c.line, le.Kind, le.Ambiguous(), err, c.kind, c.amb)
		}
		if le.Error() == "" {
			t.Errorf("%q: empty error text", c.line)
		}
	}
}

// ---------------------------------------------------------------------------
// Divergences from the old hand-written scanner (KILN04 section 4.9 table)
// ---------------------------------------------------------------------------

// Lines the old scanner accepted at the syntax level (the signal-line cases were
// checked against trace.Parse when this table was written) and this grammar still
// accepts: the places where it is exactly
// as lenient as the scanner was.
func TestStillAccepted(t *testing.T) {
	for _, line := range []string{
		`1  --- A --- B ---`,            // a " ---" inside the text is text
		`1  --- SIGX {a="x} ---`,        // signal text is not tokenized: a lone quote is text
		`1  --- SIGX {a=[} ---`,         // and so is an unbalanced bracket
		`1  ---  ---`,                   // the opener's space and the closer's space are different spaces
		`1  f(--- , 0x1) = 0`,           // "--- " is plain text inside arguments
		`1  f(???(), 1) = 0`,            // "???(" is plain text inside arguments
		`1  f(+++ exited with 0 +++)`,   // as is an exit line
		`1  stat("a", 0x1) = 0 /* c */`, // free-form tail
		`1  +++ exited with 0007 +++`,   // leading zeros are digits
		`1  +++ killed by SIG +++`,
	} {
		if _, err := ParseLine(line, DefaultOptions()); err != nil {
			t.Errorf("%q: %v", line, err)
		}
	}
}

// The stricter-only divergences: the old scanner accepted each line below and
// this path refuses it.  Each is listed in the D24 divergence table.
func TestStricterOnlyDivergences(t *testing.T) {
	cases := []struct {
		line string
		why  string
		kind Kind
	}{
		{"1  stat(\"a\", 0x1) = 0 ", "trailing space after a call", KindSpace},
		{"1  +++ exited with 0 +++ ", "trailing space after an exit line (the old scanner refused this too)", KindSpace},
		{`1  +++ exited with +5 +++`, "a signed exit code (Atoi accepted a sign)", KindSyntax},
		{`1  +++ exited with -0 +++`, "a signed exit code", KindSyntax},
		{`1  stat("a", 0x1) = 0 "abc`, "an unterminated quote in the result tail (the old scanner: result)", KindErrToken},
		{`1  stat("a", 0x1) = 0 3 <x>`, "a bad '<' in the result tail (the old scanner: result)", KindErrToken},
		{`1  f(a)=0 <... g resumed>`, "a resume marker in the tail", KindErrToken},
	}
	for _, c := range cases {
		_, err := ParseLine(c.line, DefaultOptions())
		var le *LineError
		if !errors.As(err, &le) || le.Kind != c.kind {
			t.Errorf("%q (%s): %v, want kind %d", c.line, c.why, err, c.kind)
		}
	}
}

// Reason-only divergences: both refuse, but the grammar path cannot say which
// problem came first when a line has a structural error AND a bad token.  The
// bad token is reported (ambiguous for a bad annotation).
func TestBadTokenWinsOverEarlierSyntaxError(t *testing.T) {
	_, err := ParseLine(`1  stat("a", [0x1, 3<\q>) = 0`, DefaultOptions())
	var le *LineError
	if !errors.As(err, &le) || le.Kind != KindBadAnnotation || !le.Ambiguous() {
		t.Fatalf("got %v", err)
	}
}

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

func TestDepthLimitNeverCrashes(t *testing.T) {
	for _, n := range []int{10, 100, 1000, 20000} {
		line := `1  f(` + strings.Repeat("[", n) + strings.Repeat("]", n) + `)`
		_, _, err := ParseLineTokens(line, DefaultOptions())
		var le *LineError
		if n >= 100 {
			if !errors.As(err, &le) || le.Kind != KindLimit || !errors.Is(err, parser.ErrMaxDepth) {
				t.Errorf("depth %d: %v", n, err)
			}
		}
	}
	// Even with no explicit limit the parser's own default depth applies.
	line := `1  f(` + strings.Repeat("[", 300000) + `)`
	if _, err := ParseLine(line, Options{Options: parser.Options{StartRule: StartRule, MaxDepth: 50000}}); err == nil {
		t.Error("deeply nested line was accepted")
	}
}

// Pin the nesting depth the default options accept: realistic structs nest
// 6 or 7 levels (msg_iov inside msghdr inside an array), and each bracket
// level costs a fixed number of rule levels.
func TestAcceptedNestingDepth(t *testing.T) {
	try := func(n int) error {
		line := `1  f(` + strings.Repeat("[", n) + strings.Repeat("]", n) + `)`
		_, _, err := ParseLineTokens(line, DefaultOptions())
		return err
	}
	if err := try(MaxBracketNesting); err != nil {
		t.Errorf("nesting %d refused: %v", MaxBracketNesting, err)
	}
	var le *LineError
	if err := try(MaxBracketNesting + 1); !errors.As(err, &le) || le.Kind != KindLimit {
		t.Errorf("nesting %d: %v", MaxBracketNesting+1, err)
	}
	// Mixed bracket kinds and nested calls cost the same per level.
	mixed := `1  f(` + strings.Repeat("{", 31) + strings.Repeat("g(", 33) + strings.Repeat(")", 33) + strings.Repeat("}", 31) + `)`
	if _, _, err := ParseLineTokens(mixed, DefaultOptions()); err != nil {
		t.Errorf("mixed nesting 64: %v", err)
	}
}

func TestTokenLimit(t *testing.T) {
	// 2 tokens per "a," so 40000 args are 80001+ tokens.
	line := `1  f(` + strings.Repeat("a,", 40000) + `a)`
	_, _, err := ParseLineTokens(line, DefaultOptions())
	var le *LineError
	if !errors.As(err, &le) || le.Kind != KindLimit || !errors.Is(err, parser.ErrMaxTokens) {
		t.Fatalf("got %v", err)
	}
	// Just under the cap parses: PID + SYSCALL + RPAREN + EOF + (a,)*n + a.
	n := (DefaultMaxTokens - 5) / 2
	line = `1  f(` + strings.Repeat("a,", n) + `a)`
	if _, _, err := ParseLineTokens(line, DefaultOptions()); err != nil {
		t.Fatalf("under the cap: %v", err)
	}
	// A memo limit is a limit error too.
	o := DefaultOptions()
	o.MaxMemoSlots = 10
	if _, _, err := ParseLineTokens(`1  f(a, b, c) = 0`, o); err == nil {
		t.Error("memo limit not applied")
	} else if !errors.As(err, &le) || le.Kind != KindLimit {
		t.Errorf("memo limit: %v", err)
	}
}

func TestUnknownStartRule(t *testing.T) {
	_, _, err := ParseLineTokens(`1  f()`, Options{Options: parser.Options{StartRule: "nope", MaxDepth: 8}})
	if err == nil {
		t.Fatal("unknown start rule accepted")
	}
	var le *LineError
	if !errors.As(err, &le) || le.Kind != KindSyntax {
		t.Errorf("got %v", err)
	}
}

// ---------------------------------------------------------------------------
// The grammar never accepts what the lexer marks as an error
// ---------------------------------------------------------------------------

func TestErrTokenNeverParses(t *testing.T) {
	c, _ := Compiled()
	for _, line := range []string{`1  f("a`, `1  f(3<a`, `1  f(3 <a>)`, "1  f(\t)", `1  <... x`} {
		toks, err := stracelexer.TokenizeLine(line)
		if err != nil {
			t.Fatal(err)
		}
		if _, _, ok := firstErr(toks); !ok {
			t.Fatalf("%q: no error token", line)
		}
		if _, err := c.Parse(toks, DefaultOptions().Options); err == nil {
			t.Errorf("%q: the raw grammar accepted a line with an ERR token", line)
		}
	}
}

func firstErr(toks []lexer.Token) (lexer.Token, int, bool) { return stracelexer.FirstError(toks) }

// ---------------------------------------------------------------------------
// Fixtures and the corpus of every line shape kiln's tests use
// ---------------------------------------------------------------------------

func lines(t testing.TB, path string) []string {
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var out []string
	for _, l := range strings.Split(string(b), "\n") {
		if l != "" {
			out = append(out, l)
		}
	}
	return out
}

// Every line of kiln's committed .strace fixtures parses.
func TestFixturesParse(t *testing.T) {
	files, _ := filepath.Glob("testdata/*.strace")
	if len(files) < 4 {
		t.Fatalf("fixtures missing: %v", files)
	}
	n := 0
	for _, f := range files {
		for _, l := range lines(t, f) {
			n++
			if _, err := ParseLine(l, DefaultOptions()); err != nil {
				t.Errorf("%s: %q: %v", f, l, err)
			}
		}
	}
	if n < 60 {
		t.Errorf("only %d fixture lines", n)
	}
}

// corpus.txt is every strace-looking line in kiln's tests (valid and invalid,
// including fragments that tests complete with fmt.Sprintf).  The lines this
// grammar refuses are pinned in corpus_refused.txt, each one reviewed: they are
// fragments, or lines kiln's own tests expect to be refused.  Any change to
// the grammar that changes the set fails here, so a change is always a review.
func TestCorpusAcceptance(t *testing.T) {
	var refused []string
	all := lines(t, "testdata/corpus.txt")
	if len(all) < 300 {
		t.Fatalf("corpus has %d lines", len(all))
	}
	for _, l := range all {
		if _, err := ParseLine(l, DefaultOptions()); err != nil {
			refused = append(refused, l)
		}
	}
	want := lines(t, "testdata/corpus_refused.txt")
	if !reflect.DeepEqual(refused, want) {
		t.Errorf("refused set changed:\n got  %d lines\n want %d lines", len(refused), len(want))
		have := map[string]bool{}
		for _, l := range want {
			have[l] = true
		}
		for _, l := range refused {
			if !have[l] {
				t.Errorf("newly refused: %q", l)
			}
		}
		got := map[string]bool{}
		for _, l := range refused {
			got[l] = true
		}
		for _, l := range want {
			if !got[l] {
				t.Errorf("no longer refused: %q", l)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Fuzz and properties
// ---------------------------------------------------------------------------

func checkParsed(t testing.TB, line string) {
	ast, toks, err := ParseLineTokens(line, DefaultOptions())
	if err != nil {
		var le *LineError
		if !errors.As(err, &le) {
			t.Fatalf("%q: %v is not a LineError", line, err)
		}
		return
	}
	if BodyKind(ast) == "" {
		t.Fatalf("%q: accepted without a body", line)
	}
	if _, _, bad := stracelexer.FirstError(toks); bad {
		t.Fatalf("%q: accepted with an ERR token", line)
	}
	for _, a := range RawArgs(line, ast) {
		if a == "" || a[0] == ' ' || a[len(a)-1] == ' ' || !strings.Contains(line, a) {
			t.Fatalf("%q: bad raw argument %q", line, a)
		}
	}
	_ = TextAfterClose(line, ast)
}

func FuzzParseLine(f *testing.F) {
	for _, l := range lines(f, "testdata/corpus.txt") {
		f.Add(l)
	}
	f.Fuzz(func(t *testing.T, line string) { checkParsed(t, line) })
}

func TestParseNeverPanicsOnRandomInput(t *testing.T) {
	alpha := []string{"1  ", "f(", ")", "[", "]", "{", "}", ",", " ", "<", ">", "\"", "\\", "3</x>", "<unfinished ...>", "<... f resumed>",
		"+++ exited with 0 +++", "--- ", " ---", "a", "=", "|", "x=", "\t", "???(", "...", "(", "O_RDONLY", "-1"}
	seed := uint32(7)
	next := func(n int) int {
		seed = seed*1664525 + 1013904223
		return int(seed>>8) % n
	}
	for i := 0; i < 20000; i++ {
		var sb strings.Builder
		sb.WriteString("1  ")
		for k := next(25); k >= 0; k-- {
			sb.WriteString(alpha[next(len(alpha))])
		}
		checkParsed(t, sb.String())
	}
}

// Parsing is deterministic and the tree does not alias the token slice.
func TestParseTwiceSame(t *testing.T) {
	l := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`
	a, _, _ := ParseLineTokens(l, DefaultOptions())
	b, _, _ := ParseLineTokens(l, DefaultOptions())
	if !reflect.DeepEqual(a, b) {
		t.Error("two parses differ")
	}
}

// ---------------------------------------------------------------------------
// Benchmarks (the D24 gate)
// ---------------------------------------------------------------------------

func benchLines(b *testing.B) ([]string, int) {
	var out []string
	total := 0
	files, _ := filepath.Glob("testdata/*.strace")
	for _, f := range files {
		for _, l := range lines(b, f) {
			out = append(out, l)
			total += len(l) + 1
		}
	}
	return out, total
}

// BenchmarkLexParseFixtures is lex + parse of kiln's four fixtures, one line at
// a time.  MB/s is bytes of trace per second.
func BenchmarkLexParseFixtures(b *testing.B) {
	ls, total := benchLines(b)
	opts := DefaultOptions()
	b.SetBytes(int64(total))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, l := range ls {
			if _, _, err := ParseLineTokens(l, opts); err != nil {
				b.Fatal(err)
			}
		}
	}
}

// BenchmarkLexOnlyFixtures is the lexer alone, for the split.
func BenchmarkLexOnlyFixtures(b *testing.B) {
	ls, total := benchLines(b)
	b.SetBytes(int64(total))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, l := range ls {
			if _, err := stracelexer.TokenizeLine(l); err != nil {
				b.Fatal(err)
			}
		}
	}
}
