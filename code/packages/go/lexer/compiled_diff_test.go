package lexer

// Differential tests: Compiled engine vs the frozen legacy oracle
// ================================================================
//
// The compiled engine (first-byte dispatch, literal fast path, deterministic
// matchers) is an optimisation. Its contract is that it is *indistinguishable*
// from the simple interpreter it replaced for every input the interpreter
// accepted: same types, values, lines, columns and flags.
//
// legacy_oracle_test.go holds a frozen copy of that interpreter. These tests
// run both over real grammars from code/grammars and over many inputs:
//
//   - hand-written samples in each language,
//   - every prefix of the short samples (truncation exercises "unterminated"),
//   - random byte-level mutations (inserted quotes, newlines, tabs, multi-byte
//     runes and invalid UTF-8), and
//   - a Go fuzz target (FuzzCompiledMatchesLegacy) so `go test -fuzz` can keep
//     hunting.
//
// Where the oracle panics, the compiled engine must return an error with the
// same text rather than panic.

import (
	"errors"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"reflect"
	"sync"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// grammarCase names a real grammar and gives sample sources for it.
type grammarCase struct {
	name    string
	file    string // relative to code/grammars, or "" when tokens is set
	samples []string
}

// inlineTokens holds grammars given as source text instead of a file (file == "").
var inlineTokens = map[string]string{"strace-like": straceLikeTokens}

var diffCases = []grammarCase{
	{"strace-like", "", append([]string{straceLikeInput(1)}, straceLikeLines...)},
	{"json", "json/json.tokens", []string{
		`{"a": [1, 2.5, -3e+10, true, false, null], "b": {"c": "d\"e\\f\u00e9"}}`,
		"[\n  \"x\",\n  \"y\"\n]\n",
		jsonDocument(3),
		`"unterminated`,
		`{"k": 01.}`,
	}},
	{"c", "c/c.tokens", []string{
		"#include <stdio.h>\nint main(int argc, char **argv) {\n  printf(\"hi %d\\n\", argc);\n  return 0; /* done */\n}\n",
		"static const char *s = \"a\\\"b\"; char c = '\\n'; // comment\nx->y += 0x1F << 3;\n",
		"unsigned long long v = 123456789ULL; float f = 1.5e-3f;",
	}},
	{"python", "python/python.tokens", []string{
		"def f(x):\n    if x:\n        return [1, 2, 3]\n    return {'a': x}\n",
		"class A:\n\tpass\n",
		"x = 1\n  y = 2\n",
		"# comment only\nprint('hi')  # trailing\n\nfor i in range(3):\n    print(i)\n",
		"s = '''triple\nquoted''' + \"dq\"\n",
		"if (a and\n    b):\n    pass",
	}},
	{"css", "css/css.tokens", []string{
		"a:hover > b { color: #fff; margin: 0 auto; background: url(\"x.png\"); }\n@media (max-width: 10px) { .c { width: 10px; } }\n",
		"/* c */ h1, h2 { font: 12px/1.5 \"Helvetica Neue\", sans-serif !important; }",
	}},
	{"xml", "xml/xml.tokens", []string{
		"<?xml version=\"1.0\"?>\n<a b=\"c\"><!-- hi --><![CDATA[ x < y ]]><d/>text &amp; more</a>\n",
	}},
	{"javascript", "javascript/javascript.tokens", []string{
		"const f = (a, b) => a + b; let async = 1; get = 2;\n",
	}},
	{"java8", "java/java8.tokens", []string{
		"public class A { private static final int X = 0x1F; String s = \"a\\\"b\"; char c = 'x'; // c\n void f() { /* b */ } }\n",
	}},
	{"lisp", "lisp/lisp.tokens", []string{
		"(define (f x) (if (< x 2) 1 (* x (f (- x 1))))) ; c\n'(a . b) \"s\"\n",
	}},
	{"dartmouth_basic", "dartmouth_basic/dartmouth_basic.tokens", []string{
		"10 LET X = 1\n20 PRINT \"HELLO\"; X\n30 FOR I = 1 TO 10 STEP 2\n40 NEXT I\n50 GOTO 10\n60 end\n",
	}},
	{"idl", "idl/idl.tokens", []string{
		"module M { interface I { long f(in string s); }; };\n",
	}},
	{"cobol", "cobol/cobol.tokens", []string{
		"       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n       PROCEDURE DIVISION.\n           DISPLAY 'Hello'.\n           STOP RUN.\n",
		"       01 WS-A PIC 9(4) VALUE 12.\n       MOVE 1 TO WS-A.\n",
	}},
	{"haskell98", "haskell/haskell98.tokens", []string{
		"module Main where\nimport Data.List\nmain :: IO ()\nmain = do\n  let x = 1\n      y = 2\n  print (x + y)\n",
		"f x = case x of\n  Just y -> y\n  Nothing -> 0 -- c\n{- block -}\n",
	}},
	{"sql", "sql/sql.tokens", []string{
		"SELECT a, b FROM t WHERE x = 'it''s' AND y >= 10 ORDER BY a DESC;\n",
		"select * from users u join orders o on u.id = o.uid -- trailing\n",
	}},
	{"dot", "dot/dot.tokens", []string{
		"digraph G { a -> b [label=\"x\"]; b -> c; }\n",
	}},
}

// loadedGrammar is a parsed grammar with its compiled form, cached across tests.
type loadedGrammar struct {
	name     string
	grammar  *grammartools.TokenGrammar
	compiled *Compiled
	samples  []string
}

var (
	loadOnce   sync.Once
	loadedAll  []*loadedGrammar
	loadedErrs []string
)

func loadAllGrammars() ([]*loadedGrammar, []string) {
	loadOnce.Do(func() {
		for _, gc := range diffCases {
			src := inlineTokens[gc.name]
			if gc.file != "" {
				path := filepath.Join("..", "..", "..", "grammars", filepath.FromSlash(gc.file))
				data, err := os.ReadFile(path)
				if err != nil {
					loadedErrs = append(loadedErrs, fmt.Sprintf("%s: %v", gc.name, err))
					continue
				}
				src = string(data)
			}
			g, err := grammartools.ParseTokenGrammar(src)
			if err != nil {
				loadedErrs = append(loadedErrs, fmt.Sprintf("%s: parse: %v", gc.name, err))
				continue
			}
			c, err := Compile(g)
			if err != nil {
				loadedErrs = append(loadedErrs, fmt.Sprintf("%s: compile: %v", gc.name, err))
				continue
			}
			loadedAll = append(loadedAll, &loadedGrammar{name: gc.name, grammar: g, compiled: c, samples: gc.samples})
		}
	})
	return loadedAll, loadedErrs
}

// oracleOutcome runs the frozen legacy engine, converting a panic to a string.
func oracleOutcome(g *grammartools.TokenGrammar, src string) (toks []Token, panicMsg string, ok bool) {
	defer func() {
		if r := recover(); r != nil {
			toks, panicMsg, ok = nil, fmt.Sprint(r), true
		}
	}()
	l := newOracleLexer(src, g)
	if l == nil {
		return nil, "", false
	}
	return l.Tokenize(), "", true
}

// compareOne checks one (grammar, input) pair and reports a mismatch.
func compareOne(lg *loadedGrammar, src string) error {
	want, panicMsg, ok := oracleOutcome(lg.grammar, src)
	if !ok {
		return fmt.Errorf("oracle could not build a lexer")
	}
	got, err := lg.compiled.Tokenize(src)
	if panicMsg != "" {
		if err == nil {
			return fmt.Errorf("oracle panicked (%s) but compiled accepted", panicMsg)
		}
		if err.Error() != panicMsg {
			return fmt.Errorf("error text differs: oracle %q vs compiled %q", panicMsg, err.Error())
		}
		var le *LexError
		if !errors.As(err, &le) {
			return fmt.Errorf("error is not a *LexError: %T", err)
		}
		return nil
	}
	if err != nil {
		return fmt.Errorf("oracle accepted but compiled errored: %v", err)
	}
	if len(want) != len(got) {
		return fmt.Errorf("token count differs: oracle %d compiled %d", len(want), len(got))
	}
	for i := range want {
		if want[i] != got[i] {
			return fmt.Errorf("token %d differs: oracle %+v compiled %+v", i, want[i], got[i])
		}
	}
	// The legacy wrapper must agree as well (it shares the engine).
	legacy := NewGrammarLexer(src, lg.grammar).Tokenize()
	if !reflect.DeepEqual(legacy, got) {
		return fmt.Errorf("NewGrammarLexer().Tokenize() differs from Compiled.Tokenize()")
	}
	return nil
}

func TestDifferentialGrammarsLoad(t *testing.T) {
	all, errs := loadAllGrammars()
	for _, e := range errs {
		t.Errorf("grammar did not load: %s", e)
	}
	if len(all) != len(diffCases) {
		t.Fatalf("loaded %d of %d grammars", len(all), len(diffCases))
	}
}

func TestDifferentialSamples(t *testing.T) {
	all, _ := loadAllGrammars()
	for _, lg := range all {
		for i, src := range lg.samples {
			if err := compareOne(lg, src); err != nil {
				t.Errorf("%s sample %d: %v\ninput: %q", lg.name, i, err, src)
			}
		}
	}
}

func TestDifferentialPrefixes(t *testing.T) {
	all, _ := loadAllGrammars()
	for _, lg := range all {
		for i, src := range lg.samples {
			if len(src) > 160 {
				src = src[:160]
			}
			for n := 0; n <= len(src); n++ {
				if err := compareOne(lg, src[:n]); err != nil {
					t.Fatalf("%s sample %d prefix %d: %v\ninput: %q", lg.name, i, n, err, src[:n])
				}
			}
		}
	}
}

// mutationPool is the material inserted by the mutation test.
var mutationPool = []string{
	"\"", "'", "\n", "\t", "\r", " ", "\\", "{", "}", "[", "]", "(", ")", "<", ">",
	"/", "*", "#", "é", "日本", "\xff", "\xc3", "\x00", "0x", "-", ".", "`", "$", "e+",
	"/*", "*/", "//", "\"\"\"", "<!--", "-->", "<![CDATA[", "]]>",
}

func mutate(r *rand.Rand, s string) string {
	b := []byte(s)
	for k := 0; k < 1+r.Intn(3); k++ {
		switch r.Intn(4) {
		case 0: // delete a span
			if len(b) > 0 {
				i := r.Intn(len(b))
				j := i + r.Intn(4)
				if j > len(b) {
					j = len(b)
				}
				b = append(b[:i], b[j:]...)
			}
		case 1: // insert
			ins := mutationPool[r.Intn(len(mutationPool))]
			i := 0
			if len(b) > 0 {
				i = r.Intn(len(b) + 1)
			}
			b = append(b[:i], append([]byte(ins), b[i:]...)...)
		case 2: // replace one byte
			if len(b) > 0 {
				b[r.Intn(len(b))] = byte(r.Intn(256))
			}
		case 3: // duplicate a span
			if len(b) > 1 {
				i := r.Intn(len(b))
				j := i + r.Intn(len(b)-i)
				b = append(b[:j], append(append([]byte{}, b[i:j]...), b[j:]...)...)
			}
		}
	}
	return string(b)
}

func TestDifferentialMutations(t *testing.T) {
	all, _ := loadAllGrammars()
	rounds := 120
	if testing.Short() {
		rounds = 20
	}
	for gi, lg := range all {
		r := rand.New(rand.NewSource(int64(1000 + gi)))
		for i := 0; i < rounds; i++ {
			for _, src := range lg.samples {
				m := mutate(r, src)
				if err := compareOne(lg, m); err != nil {
					t.Fatalf("%s: %v\ninput: %q", lg.name, err, m)
				}
			}
		}
	}
}

// FuzzCompiledMatchesLegacy lets `go test -fuzz=FuzzCompiledMatchesLegacy`
// search for any input on which the compiled engine and the legacy engine
// disagree. As a plain test it replays the seed corpus.
func FuzzCompiledMatchesLegacy(f *testing.F) {
	all, _ := loadAllGrammars()
	for gi, lg := range all {
		for _, s := range lg.samples {
			f.Add(uint8(gi), s)
		}
	}
	f.Fuzz(func(t *testing.T, which uint8, src string) {
		all, _ := loadAllGrammars()
		if len(all) == 0 {
			t.Skip("no grammars loaded")
		}
		lg := all[int(which)%len(all)]
		if err := compareOne(lg, src); err != nil {
			t.Fatalf("%s: %v\ninput: %q", lg.name, err, src)
		}
	})
}
