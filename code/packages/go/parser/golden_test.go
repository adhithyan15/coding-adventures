package parser

// Real-grammar golden tests.
//
// The differential test (differential_test.go) compares the engine with a
// frozen copy of the ORIGINAL map-memo engine, but only over small synthetic
// grammars. This file adds the other half of the guarantee: the engine's
// output on REAL grammars (JSON, TypeScript 5.8) over real source text and
// under tight limits must not change when the engine's internals do.
//
// How it works: for each case we build a canonical dump of the result (see
// canonDump) and keep a SHA-256 of it, plus a short human summary, in
// testdata/golden/*.golden. The goldens were generated from the engine as it
// stood BEFORE the flat-array rewrite (P2 PR 2); the rewritten engine has to
// reproduce every line byte for byte. A hash rather than the dump itself
// keeps the repository small (a TypeScript tree is megabytes of text); when a
// line differs the test prints the new summary, and the dump itself can be
// printed from any engine revision with GOLDEN_DUMP=<case name>.
//
// Why committed goldens and not a second frozen engine in the tree: the
// original engine is already frozen in internal/oldengine. A third copy
// (the pre-flat engine) would be 1,200 lines of dead code to keep in sync
// with nothing. The goldens cost a few KB and were produced by the engine
// that was reviewed and merged.
//
// Regenerate (only when a change is MEANT to alter output, never to make a
// failure go away):  GOLDEN_UPDATE=1 go test -run TestGolden

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// canonDump writes a canonical rendering of a parse result to w: every node's
// rule name, positions and whether Children is nil, every token's type, value,
// line and column, and the SHARING structure. The sharing part matters
// because the engine's memo hits hand out distinct *ASTNodes that share one
// Children slice: the first time a backing array is met it gets an ordinal
// (`#k`), and any later node that shares it prints `=k` instead of
// descending (its contents are by definition the same).
//
// It returns the number of nodes and tokens written.
func canonDump(w io.Writer, n *ASTNode) (nodes, toks int) {
	seen := map[*interface{}]int{}
	type frame struct {
		n    *ASTNode
		next int
	}
	var stack []frame
	// enter writes a node's header and, when it has children not yet seen,
	// pushes it so the loop below walks them. Iterative: the dump must work
	// on trees at the depth limit.
	enter := func(n *ASTNode) {
		nodes++
		fmt.Fprintf(w, "(%s@%d:%d-%d:%d", n.RuleName, n.StartLine, n.StartColumn, n.EndLine, n.EndColumn)
		if n.Children == nil {
			io.WriteString(w, " nil")
		}
		if len(n.Children) == 0 {
			io.WriteString(w, ")")
			return
		}
		p := &n.Children[0]
		if k, dup := seen[p]; dup {
			fmt.Fprintf(w, " =%d)", k)
			return
		}
		k := len(seen)
		seen[p] = k
		fmt.Fprintf(w, " #%d", k)
		stack = append(stack, frame{n: n})
	}
	enter(n)
	for len(stack) > 0 {
		f := &stack[len(stack)-1]
		if f.next == len(f.n.Children) {
			io.WriteString(w, ")")
			stack = stack[:len(stack)-1]
			continue
		}
		c := f.n.Children[f.next]
		f.next++
		switch x := c.(type) {
		case *ASTNode:
			io.WriteString(w, " ")
			enter(x)
		case lexer.Token:
			toks++
			fmt.Fprintf(w, " [%s:%q:%d:%d]", x.TypeName, x.Value, x.Line, x.Column)
		default:
			fmt.Fprintf(w, " ?%T", c)
		}
	}
	return nodes, toks
}

// errKind names the sentinel an error wraps (the part callers branch on).
func errKind(err error) string {
	switch {
	case errors.Is(err, ErrMaxDepth):
		return "depth"
	case errors.Is(err, ErrMaxTokens):
		return "tokens"
	case errors.Is(err, ErrMaxMemoSlots):
		return "memo"
	case errors.Is(err, ErrUnknownStartRule):
		return "startrule"
	}
	var ge *GrammarParseError
	if errors.As(err, &ge) {
		return "syntax"
	}
	return "other"
}

// goldenLine runs one parse and renders its result as one golden line:
//
//	name <TAB> sha256(dump) <TAB> summary
//
// For an error the "dump" is the error text, kind, nil-ness of the tree and
// offending token, and the summary shows the text. A nil tree with a nil
// error would be a bug, and is rendered so the comparison catches it.
func goldenLine(name string, c *Compiled, toks []lexer.Token, opts Options) string {
	ast, err := c.Parse(toks, opts)
	h := sha256.New()
	var summary string
	switch {
	case err != nil:
		var tok lexer.Token
		var ge *GrammarParseError
		if errors.As(err, &ge) {
			tok = ge.Tok
		}
		fmt.Fprintf(h, "ERR nil=%v kind=%s msg=%q tok=%s:%q:%d:%d", ast == nil, errKind(err), err.Error(),
			tok.TypeName, tok.Value, tok.Line, tok.Column)
		summary = fmt.Sprintf("err kind=%s nil=%v %q", errKind(err), ast == nil, err.Error())
	case ast == nil:
		summary = "NIL-TREE-NO-ERROR"
		h.Write([]byte(summary))
	default:
		nodes, ntok := canonDump(h, ast)
		summary = fmt.Sprintf("ok nodes=%d tokens=%d", nodes, ntok)
		if want := os.Getenv("GOLDEN_DUMP"); want != "" && want == name {
			canonDump(os.Stderr, ast)
			fmt.Fprintln(os.Stderr)
		}
	}
	return name + "\t" + hex.EncodeToString(h.Sum(nil)) + "\t" + summary
}

// checkGolden compares lines with testdata/golden/<file>, or rewrites the
// file when GOLDEN_UPDATE is set.
func checkGolden(t *testing.T, file string, lines []string) {
	t.Helper()
	path := filepath.Join("testdata", "golden", file)
	got := strings.Join(lines, "\n") + "\n"
	if os.Getenv("GOLDEN_UPDATE") != "" {
		if err := os.WriteFile(path, []byte(got), 0o644); err != nil {
			t.Fatal(err)
		}
		return
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("%v (generate with GOLDEN_UPDATE=1)", err)
	}
	want := map[string]string{}
	for _, l := range strings.Split(strings.TrimSuffix(string(raw), "\n"), "\n") {
		name, _, _ := strings.Cut(l, "\t")
		want[name] = l
	}
	have := map[string]bool{}
	for _, l := range lines {
		name, _, _ := strings.Cut(l, "\t")
		have[name] = true
		if want[name] != l {
			t.Errorf("golden mismatch\n want %s\n  got %s", want[name], l)
		}
	}
	for name := range want {
		if !have[name] {
			t.Errorf("golden case %q no longer produced", name)
		}
	}
}

// realGrammar is a grammar from code/grammars compiled together with the
// lexer built from its .tokens file, exactly as the json-parser and
// typescript-parser packages do (those embed the same files as Go data;
// importing them here would be an import cycle).
type realGrammar struct {
	c   *Compiled
	tg  *grammartools.TokenGrammar
	tok func(src string) []lexer.Token
}

func loadReal(t testing.TB, dir, base string) *realGrammar {
	t.Helper()
	root := filepath.Join("..", "..", "..", "grammars", dir)
	pg, err := os.ReadFile(filepath.Join(root, base+".grammar"))
	if err != nil {
		t.Fatal(err)
	}
	tk, err := os.ReadFile(filepath.Join(root, base+".tokens"))
	if err != nil {
		t.Fatal(err)
	}
	g, err := grammartools.ParseParserGrammar(string(pg))
	if err != nil {
		t.Fatal(err)
	}
	tg, err := grammartools.ParseTokenGrammar(string(tk))
	if err != nil {
		t.Fatal(err)
	}
	c, err := CompileGrammar(g)
	if err != nil {
		t.Fatal(err)
	}
	return &realGrammar{c: c, tg: tg, tok: func(src string) []lexer.Token {
		return lexer.NewGrammarLexer(src, tg).Tokenize()
	}}
}

// goldenSources lists testdata/golden/<prefix>*.src files in name order.
func goldenSources(t testing.TB, prefix string) (names []string, srcs []string) {
	t.Helper()
	files, err := filepath.Glob(filepath.Join("testdata", "golden", prefix+"*.src"))
	if err != nil || len(files) == 0 {
		t.Fatalf("no %s*.src golden sources (%v)", prefix, err)
	}
	sort.Strings(files)
	for _, f := range files {
		b, err := os.ReadFile(f)
		if err != nil {
			t.Fatal(err)
		}
		names = append(names, strings.TrimSuffix(filepath.Base(f), ".src"))
		srcs = append(srcs, string(b))
	}
	return
}

// limitOptions are the settings every real source is also parsed under: they
// make the depth, token and memo limits fire at different places in a real
// parse (including mid-parse, where backtracking could swallow a refusal if
// the sticky flag were wrong).
var limitOptions = []struct {
	name string
	o    Options
}{
	{"default", Options{}},
	{"depth6", Options{MaxDepth: 6}},
	{"depth14", Options{MaxDepth: 14}},
	{"depth40", Options{MaxDepth: 40}},
	{"memo50", Options{MaxMemoSlots: 50}},
	{"memo700", Options{MaxMemoSlots: 700}},
	{"tokens9", Options{MaxTokens: 9}},
}

func realGolden(t *testing.T, dir, base, prefix, file string) {
	rg := loadReal(t, dir, base)
	names, srcs := goldenSources(t, prefix)
	var lines []string
	for i, src := range srcs {
		toks := rg.tok(src)
		for _, lo := range limitOptions {
			lines = append(lines, goldenLine(names[i]+"/"+lo.name, rg.c, toks, lo.o))
		}
		// No trailing EOF: the synthetic-EOF path on a real grammar.
		if n := len(toks); n > 0 && toks[n-1].TypeName == "EOF" {
			lines = append(lines, goldenLine(names[i]+"/noeof", rg.c, toks[:n-1], Options{}))
		}
		// A truncated list: a syntax error at a real, mid-grammar position.
		if n := len(toks); n > 6 {
			lines = append(lines, goldenLine(names[i]+"/trunc", rg.c, toks[:n/2], Options{}))
		}
	}
	lines = append(lines, goldenLine("empty/default", rg.c, nil, Options{}))
	checkGolden(t, file, lines)
}

func TestGoldenJSON(t *testing.T) { realGolden(t, "json", "json", "json_", "json.golden") }

func TestGoldenTypeScript(t *testing.T) {
	realGolden(t, "typescript", "ts5.8", "ts_", "ts.golden")
}
