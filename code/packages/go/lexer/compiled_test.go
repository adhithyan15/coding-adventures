package lexer

// Tests for Compile / Compiled.Tokenize
// =====================================
//
// Three themes:
//
//  1. Errors instead of panics: every input that makes the legacy lexer panic
//     must make Compiled.Tokenize return a *LexError with the right position.
//  2. Equivalence: hand-built grammars that exercise the corners of the engine
//     (modes, case folding, escapes, keywords, odd patterns) are run through
//     both the compiled engine and the frozen legacy oracle.
//  3. Reuse: a Compiled value is immutable and safe for concurrent use.

import (
	"errors"
	"fmt"
	"strings"
	"sync"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// assertEquivalent compiles g and checks every input against the oracle.
func assertEquivalent(t *testing.T, g *grammartools.TokenGrammar, inputs ...string) {
	t.Helper()
	c, err := Compile(g)
	if err != nil {
		t.Fatalf("Compile: %v", err)
	}
	lg := &loadedGrammar{name: "inline", grammar: g, compiled: c}
	for _, in := range inputs {
		if err := compareOne(lg, in); err != nil {
			t.Errorf("input %q: %v", in, err)
		}
	}
}

func def(name, pattern string, isRegex bool) grammartools.TokenDefinition {
	return grammartools.TokenDefinition{Name: name, Pattern: pattern, IsRegex: isRegex}
}

// ---------------------------------------------------------------------------
// Compile
// ---------------------------------------------------------------------------

func TestCompileNilGrammar(t *testing.T) {
	if _, err := Compile(nil); err == nil {
		t.Fatal("expected an error for a nil grammar")
	}
}

func TestCompileReportsBadPatternsAsErrors(t *testing.T) {
	cases := []struct {
		name string
		g    *grammartools.TokenGrammar
		want string
	}{
		{"token", &grammartools.TokenGrammar{CaseSensitive: true,
			Definitions: []grammartools.TokenDefinition{def("BAD", `(`, true)}},
			"Failed to compile pattern for token BAD"},
		{"skip", &grammartools.TokenGrammar{CaseSensitive: true,
			SkipDefinitions: []grammartools.TokenDefinition{def("WS", `[`, true)}},
			"Failed to compile skip pattern WS"},
		{"group", &grammartools.TokenGrammar{CaseSensitive: true,
			Groups: map[string]*grammartools.PatternGroup{
				"g": {Name: "g", Definitions: []grammartools.TokenDefinition{def("X", `[a-`, true)}},
			}},
			"Failed to compile group pattern g/X"},
		// Would be valid if wrapped in (?:...): the stray ")" would close the
		// wrapper. The legacy engine rejected it, so must Compile.
		{"wrapper escape", &grammartools.TokenGrammar{CaseSensitive: true,
			Definitions: []grammartools.TokenDefinition{def("INJ", `)*0|0(`, true)}},
			"Failed to compile pattern for token INJ"},
		{"invalid utf-8 literal", &grammartools.TokenGrammar{CaseSensitive: true,
			Definitions: []grammartools.TokenDefinition{def("L", "\xff", false)}},
			"Failed to compile pattern for token L"},
	}
	for _, tc := range cases {
		_, err := Compile(tc.g)
		if err == nil || !strings.Contains(err.Error(), tc.want) {
			t.Errorf("%s: error %v, want it to contain %q", tc.name, err, tc.want)
		}
		// Legacy compatibility: the constructor still fails (nil lexer).
		if l := NewGrammarLexer("x", tc.g); l != nil {
			t.Errorf("%s: NewGrammarLexer should yield nil for a bad grammar", tc.name)
		}
	}
}

func TestCompileErrorsAreDeterministicAcrossGroups(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Groups: map[string]*grammartools.PatternGroup{
			"b": {Name: "b", Definitions: []grammartools.TokenDefinition{def("X", `(`, true)}},
			"a": {Name: "a", Definitions: []grammartools.TokenDefinition{def("Y", `[`, true)}},
		}}
	for i := 0; i < 20; i++ {
		_, err := Compile(g)
		if err == nil || !strings.Contains(err.Error(), "a/Y") {
			t.Fatalf("expected the error for group a first, got %v", err)
		}
	}
}

// A pattern whose `^(?:...)` wrapped form is invalid but whose bare `^pattern`
// form is valid (an unterminated \Q) must still behave like the legacy engine.
func TestCompileLegacyAnchorFallback(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{
			def("Q", `\Qab`, true),
			def("W", `[a-z]+`, true),
		}}
	c, err := Compile(g)
	if err != nil {
		t.Fatal(err)
	}
	if r := c.tables["default"].byByte['a'][0]; !r.legacyAnchor {
		t.Fatal("expected the legacy-anchor fallback rule")
	}
	assertEquivalent(t, g, "ab", "abab", "xab", "a", "ab ab", "zzab")
}

// ---------------------------------------------------------------------------
// Errors, not panics
// ---------------------------------------------------------------------------

func mustLexError(t *testing.T, err error) *LexError {
	t.Helper()
	var le *LexError
	if !errors.As(err, &le) {
		t.Fatalf("want *LexError, got %T: %v", err, err)
	}
	return le
}

func TestTokenizeUnexpectedCharacterReturnsError(t *testing.T) {
	g := mustGrammar(t, "NAME = /[a-z]+/\n")
	c, err := Compile(g)
	if err != nil {
		t.Fatal(err)
	}
	toks, err := c.Tokenize("ab\n  c §")
	if toks != nil {
		t.Errorf("tokens should be nil on error, got %v", toks)
	}
	le := mustLexError(t, err)
	if le.Line != 2 || le.Column != 5 {
		t.Errorf("position %d:%d, want 2:5", le.Line, le.Column)
	}
	if !strings.Contains(le.Message, "Unexpected character") {
		t.Errorf("message %q", le.Message)
	}
	if !strings.HasPrefix(le.Error(), "LexerError at 2:5:") {
		t.Errorf("Error() = %q", le.Error())
	}
	// The legacy wrapper panics with the same text.
	defer func() {
		r := recover()
		if r == nil || fmt.Sprint(r) != le.Error() {
			t.Errorf("legacy panic %v, want %q", r, le.Error())
		}
	}()
	NewGrammarLexer("ab\n  c §", g).Tokenize()
}

func TestTokenizeReservedKeywordReturnsError(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions:      []grammartools.TokenDefinition{def("NAME", `[a-zA-Z_]+`, true)},
		ReservedKeywords: []string{"class"},
	}
	c, _ := Compile(g)
	_, err := c.Tokenize("ok\nclass")
	le := mustLexError(t, err)
	if le.Line != 2 || le.Column != 1 || !strings.Contains(le.Message, "Reserved keyword") {
		t.Errorf("got %+v", le)
	}
	assertEquivalent(t, g, "ok", "class", "x class")
}

func indentGrammar() *grammartools.TokenGrammar {
	return &grammartools.TokenGrammar{CaseSensitive: true, Mode: "indentation",
		Definitions: []grammartools.TokenDefinition{
			def("NAME", `[a-z]+`, true), def("COLON", ":", false),
			def("LPAREN", "(", false), def("RPAREN", ")", false),
		},
		SkipDefinitions: []grammartools.TokenDefinition{
			def("WS", `[ ]+`, true), def("COMMENT", `#[^\n]*`, true),
		},
	}
}

func TestTokenizeIndentationErrors(t *testing.T) {
	c, _ := Compile(indentGrammar())

	_, err := c.Tokenize("a:\n\tb\n")
	le := mustLexError(t, err)
	if !strings.Contains(le.Message, "Tab character") || le.Line != 2 {
		t.Errorf("tab error: %+v", le)
	}

	_, err = c.Tokenize("a:\n    b\n  c\n")
	le = mustLexError(t, err)
	if !strings.Contains(le.Message, "Inconsistent dedent") || le.Line != 3 {
		t.Errorf("dedent error: %+v", le)
	}

	_, err = c.Tokenize("a §\n")
	le = mustLexError(t, err)
	if !strings.Contains(le.Message, "Unexpected character") {
		t.Errorf("unexpected char: %+v", le)
	}

	assertEquivalent(t, indentGrammar(),
		"a:\n    b\n    c\nd\n", "a:\n\tb\n", "a:\n    b\n  c\n", "a §", "# c\na: # t\n    b(\n   c)\n",
		"", "\n\n", "   \n", "a:\n    b\n\n    # only\n    c\n")
}

func TestTokenizeEmptyMatchIsAnError(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{def("OPT", `a*`, true)},
	}
	c, _ := Compile(g)
	if toks, err := c.Tokenize("aa"); err != nil || len(toks) != 2 {
		t.Fatalf("non-empty match must still work: %v %v", toks, err)
	}
	_, err := c.Tokenize("aab")
	le := mustLexError(t, err)
	if !strings.Contains(le.Message, "matched the empty string") || le.Column != 3 {
		t.Errorf("got %+v", le)
	}

	skip := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions:     []grammartools.TokenDefinition{def("X", `x`, true)},
		SkipDefinitions: []grammartools.TokenDefinition{def("WS", `[ ]*`, true)},
	}
	c, _ = Compile(skip)
	_, err = c.Tokenize("x y")
	le = mustLexError(t, err)
	if !strings.Contains(le.Message, "Skip pattern WS matched the empty string") {
		t.Errorf("skip: %+v", le)
	}

	// The same through the indentation path.
	ig := indentGrammar()
	ig.SkipDefinitions = append(ig.SkipDefinitions, def("EMPTY", `z*`, true))
	ig.SkipDefinitions[0], ig.SkipDefinitions[2] = ig.SkipDefinitions[2], ig.SkipDefinitions[0]
	c, _ = Compile(ig)
	if _, err = c.Tokenize("a §"); err == nil {
		t.Error("expected an error")
	}
}

// ---------------------------------------------------------------------------
// Equivalence on hand-built grammars
// ---------------------------------------------------------------------------

func TestEquivalenceKeywordsAndCaseFolding(t *testing.T) {
	g := &grammartools.TokenGrammar{
		CaseInsensitive: true, // CaseSensitive left false: source is lower-cased
		Definitions: []grammartools.TokenDefinition{
			def("NAME", `[a-z_][a-z0-9_]*`, true),
			def("STRING", `'[^']*'`, true),
			def("NUMBER", `[0-9]+`, true),
			def("STAR", "*", false),
		},
		Keywords:         []string{"select", "from"},
		ReservedKeywords: []string{"drop"},
		ContextKeywords:  []string{"get"},
		SkipDefinitions:  []grammartools.TokenDefinition{def("WS", `[ \t\r\n]+`, true)},
	}
	assertEquivalent(t, g,
		"SELECT 'Alice' FROM Users", "select * from t", "Get get GET", "select 'MiXed' 12",
		"DROP", "x drop y", "", "SeLeCt")

	// Case-sensitive variant.
	g2 := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{
			def("NAME", `[A-Za-z_]+`, true),
			def("STRING_DQ", `"([^"\\]|\\.)*"`, true),
		},
		Keywords:        []string{"if", "else"},
		ContextKeywords: []string{"async"},
		SkipDefinitions: []grammartools.TokenDefinition{def("WS", `[ ]+`, true)},
	}
	g2.Definitions[1].Alias = "STRING"
	assertEquivalent(t, g2, `if "a\"b" else async X`, `"unterminated`, `"é日本\n"`)
}

func TestEquivalenceStringQuoteStripping(t *testing.T) {
	for _, escapes := range []string{"", "none"} {
		g := &grammartools.TokenGrammar{CaseSensitive: true, EscapeMode: escapes,
			Definitions: []grammartools.TokenDefinition{
				def("STRING", `"([^"\\]|\\.)*"|'([^'\\]|\\.)*'`, true),
				def("TRIPLE_STRING", `"""([^"]|"[^"]|""[^"])*"""`, true),
				def("QUOTED", `"[a-z]*"`, true),
				def("ONE_STRING", `[a-z]`, true),
				def("OTHER", `[ \n]`, true),
			},
		}
		assertEquivalent(t, g,
			`"a\nb\t\\\"c" 'x\'y' """tri\nple""" ""`, `"" ''`, `"abc"`, `a b`, `"\`)
	}
}

func fixtureTransitionGrammar() *grammartools.TokenGrammar {
	return &grammartools.TokenGrammar{CaseSensitive: true, StartMode: "default",
		Definitions: []grammartools.TokenDefinition{
			def("WORD", `[a-z]+`, true),
			def("SLASH", "/", false),
			def("QUOTE", `"`, false),
			def("LBRACE", "{", false),
			def("RBRACE", "}", false),
			def("AT", "@", false),
		},
		SkipDefinitions: []grammartools.TokenDefinition{def("WS", `[ ]+`, true)},
		Groups: map[string]*grammartools.PatternGroup{
			"div": {Name: "div", Definitions: []grammartools.TokenDefinition{def("SLASH_DIV", "/", false)}},
			"str": {Name: "str", Definitions: []grammartools.TokenDefinition{
				def("BODY", `[^"]+`, true), def("QUOTE_END", `"`, false)}},
			"raw": {Name: "raw", Definitions: []grammartools.TokenDefinition{def("RAWCH", `[^}]`, true)}},
		},
		Transitions: []grammartools.ModeTransition{
			{OnTokens: []string{"SLASH"}, InMode: "default", Actions: []grammartools.TransitionAction{{Kind: "set_mode", Target: "div"}}},
			{OnTokens: []string{"SLASH_DIV"}, Actions: []grammartools.TransitionAction{{Kind: "set_mode", Target: "default"}}},
			{OnTokens: []string{"QUOTE"}, Actions: []grammartools.TransitionAction{{Kind: "push", Target: "str"}}},
			{OnTokens: []string{"QUOTE_END"}, Actions: []grammartools.TransitionAction{{Kind: "pop"}}},
			{OnTokens: []string{"LBRACE"}, Actions: []grammartools.TransitionAction{{Kind: "disable_skip"}, {Kind: "push", Target: "raw"}}},
			{OnTokens: []string{"RBRACE"}, Actions: []grammartools.TransitionAction{{Kind: "pop"}, {Kind: "enable_skip"}}},
			{OnTokens: []string{"AT"}, Actions: []grammartools.TransitionAction{{Kind: "set_mode", Target: "nonexistent"}}},
			{OnTokens: []string{"WORD"}, OnValue: "go", Actions: []grammartools.TransitionAction{{Kind: "pop"}, {Kind: "pop"}}},
		},
	}
}

func TestEquivalenceModeTransitions(t *testing.T) {
	assertEquivalent(t, fixtureTransitionGrammar(),
		"a / b / c", `x "in str" y`, `{ raw  text } z`, "a @ b @ c", "go go", `"a "go" b"`,
		"/ /", "{}", `"`, "{ no close", "a\nb")
}

func TestEquivalenceLayoutMode(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true, Mode: "layout",
		Definitions: []grammartools.TokenDefinition{
			def("NAME", `[a-z]+`, true), def("EQ", "=", false), def("LBRACE", "{", false), def("RBRACE", "}", false),
		},
		LayoutKeywords:  []string{"where", "Let"},
		SkipDefinitions: []grammartools.TokenDefinition{def("WS", `[ \t\r]+`, true)},
	}
	assertEquivalent(t, g, "module where\n  a = b\n  c = d\ne", "x where {\n a\n}\n", "where\n\n  a\n b")
}

func TestEquivalenceLiteralWithReplacementCharacter(t *testing.T) {
	// A literal containing U+FFFD must also match an invalid UTF-8 byte, as
	// the regexp engine did; so it is excluded from the byte-prefix fast path.
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{
			def("REPL", "\uFFFD", false),
			def("WORD", `[a-z]+`, true),
		}}
	assertEquivalent(t, g, "\xff", "\uFFFD", "a\xffb", "\xc3", "é")
	if isPlainLiteral("\uFFFD") || isPlainLiteral("") || isPlainLiteral("\xff") || !isPlainLiteral("ok") {
		t.Error("isPlainLiteral misclassified")
	}
}

func TestEquivalenceDispatchOrderIsPreserved(t *testing.T) {
	// "==" must beat "=", a keyword-like regex must beat the catch-all, and
	// the catch-all must only fire when nothing earlier matches.
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{
			def("EQEQ", "==", false),
			def("EQ", "=", false),
			def("ARROW", "=>", false), // shadowed by EQ: never matches
			def("NUM", `[0-9]+`, true),
			def("ID", `[a-z][a-z0-9]*`, true),
			def("ID2", `[a-z]+`, true), // shadowed
			def("ANY", `.`, true),
		}}
	assertEquivalent(t, g, "a==b=>c=1 2 x9 é\xff\n§")
}

func TestEquivalenceMultiLineSkipTracksPosition(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{def("W", `[a-z]+`, true)},
		SkipDefinitions: []grammartools.TokenDefinition{
			def("BLOCK", `/\*([^*]|\*[^/])*\*/`, true),
			def("WS", `[ \n]+`, true),
		}}
	assertEquivalent(t, g, "a /* x\n y\n */ b\n\n   c /**/ d")
	c, _ := Compile(g)
	toks, err := c.Tokenize("a /* x\n y\n */ b")
	if err != nil {
		t.Fatal(err)
	}
	if b := toks[1]; b.Value != "b" || b.Line != 3 || b.Column != 5 {
		t.Errorf("token after multi-line comment: %+v", b)
	}
}

func TestEquivalenceStraceAndJSON(t *testing.T) {
	for _, tc := range []struct {
		name, tokens string
		inputs       []string
	}{
		{"strace", straceLikeTokens, append([]string{straceLikeInput(2)}, straceLikeLines...)},
		{"json", jsonLikeTokens, []string{jsonDocument(3), `{"a":1}`, `"\u00zz"`, `-`, `1.e`}},
	} {
		g := mustGrammar(t, tc.tokens)
		assertEquivalent(t, g, tc.inputs...)
	}
}

// ---------------------------------------------------------------------------
// Reuse and concurrency
// ---------------------------------------------------------------------------

func TestCompiledIsReusableAndConcurrentSafe(t *testing.T) {
	g := mustGrammar(t, straceLikeTokens)
	c, err := Compile(g)
	if err != nil {
		t.Fatal(err)
	}
	want, err := c.Tokenize(straceLikeInput(1))
	if err != nil {
		t.Fatal(err)
	}
	var wg sync.WaitGroup
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for k := 0; k < 20; k++ {
				got, err := c.Tokenize(straceLikeInput(1))
				if err != nil || len(got) != len(want) {
					t.Errorf("concurrent run diverged: %v", err)
					return
				}
				for j := range got {
					if got[j] != want[j] {
						t.Errorf("token %d differs", j)
						return
					}
				}
			}
		}()
	}
	wg.Wait()
}

func TestTokenCapacityHint(t *testing.T) {
	if tokenCapacityHint(0) != 2 || tokenCapacityHint(300) != 102 || tokenCapacityHint(1<<30) != 1<<16 {
		t.Error("unexpected capacity hints")
	}
}

func TestAdvanceNTracksLinesAndColumns(t *testing.T) {
	g := mustGrammar(t, "A = /./\n")
	c, _ := Compile(g)
	l := c.newLexer("ab\ncd\n\nef")
	l.advanceN(2)
	if l.line != 1 || l.column != 3 {
		t.Errorf("after 'ab': %d:%d", l.line, l.column)
	}
	l.advanceN(5) // "\ncd\n\n" spans three newlines? it is \n c d \n \n
	if l.line != 4 || l.column != 1 {
		t.Errorf("after newlines: %d:%d", l.line, l.column)
	}
	l.advanceN(2)
	if l.line != 4 || l.column != 3 {
		t.Errorf("after 'ef': %d:%d", l.line, l.column)
	}
}

func TestUnknownGroupPushStillPanicsForCallbacks(t *testing.T) {
	// The callback API is unchanged: pushing an undefined group panics.
	l := NewGrammarLexer("a", fixtureTransitionGrammar())
	ctx := &LexerContext{lexer: l, source: "a"}
	defer func() {
		r := recover()
		if r == nil || !strings.Contains(fmt.Sprint(r), "Unknown pattern group") {
			t.Errorf("got %v", r)
		}
	}()
	ctx.PushGroup("zzz")
}

// ---------------------------------------------------------------------------
// Legacy-only features keep working on top of the shared engine
// ---------------------------------------------------------------------------

func TestHooksAndCallbackLookbehind(t *testing.T) {
	g := &grammartools.TokenGrammar{CaseSensitive: true,
		Definitions: []grammartools.TokenDefinition{
			def("NAME", `[a-z]+`, true), def("LPAREN", "(", false), def("RPAREN", ")", false),
			def("LBRACKET", "[", false), def("RBRACKET", "]", false),
			def("LBRACE", "{", false), def("RBRACE", "}", false),
		}}
	l := NewGrammarLexer("A b\n(c [d {e}])", g)
	l.AddPreTokenize(func(s string) string { return strings.ToLower(s) })
	l.AddPostTokenize(func(ts []Token) []Token { return ts[:len(ts)-1] }) // drop EOF

	var prev []string
	var newlines []bool
	var depths []int
	l.SetOnToken(func(tok Token, ctx *LexerContext) {
		if p := ctx.PreviousToken(); p != nil {
			prev = append(prev, p.Value)
		} else {
			prev = append(prev, "")
		}
		newlines = append(newlines, ctx.PrecededByNewline())
		depths = append(depths, ctx.BracketDepth(""))
	})
	toks := l.Tokenize()
	if toks[len(toks)-1].TypeName == "EOF" {
		t.Fatal("post-tokenize hook did not run")
	}
	if toks[0].Value != "a" {
		t.Errorf("pre-tokenize hook did not run: %v", toks[0])
	}
	if prev[0] != "" || prev[1] != "a" {
		t.Errorf("lookbehind: %v", prev)
	}
	if !newlines[2] { // "(" follows the NEWLINE token on the previous line
		t.Errorf("PrecededByNewline: %v", newlines)
	}
	if l.BracketDepth("paren") != 0 || depths[len(depths)-1] != 0 {
		t.Errorf("bracket depths should balance: %v", depths)
	}
}
