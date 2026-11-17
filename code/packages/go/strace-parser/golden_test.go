package straceparser

// Engine-independence goldens for the strace grammar.
//
// The shared parser engine (code/packages/go/parser/compiled.go) is rewritten
// from time to time for speed. Its own tests compare it with a frozen engine
// on synthetic grammars and pin JSON and TypeScript goldens; THIS file pins
// the real consumer: every line of the strace corpus (valid and refused)
// parsed through ParseLineTokens, under the default limits and under tight
// ones that fire mid-parse, must produce exactly the tree (rules, positions,
// nil-ness of Children, tokens, sharing of memoised Children slices) or the
// exact error (kind, text, offending token) it produced when the goldens were
// generated from the pre-rewrite engine.
//
// Each case is one line of testdata/golden/strace.golden:
//
//	sha256(line)[:12] / option-set <TAB> sha256(dump) <TAB> summary
//
// keyed by the hash of the INPUT LINE rather than its position, so adding a
// line to the corpus does not shift every other case. A corpus line with no
// golden is skipped (and counted); a golden with no corpus line is skipped
// too, since another PR may have legitimately removed it. The test insists
// that at least minMatched cases still match, so it cannot quietly go
// vacuous. Regenerate with GOLDEN_UPDATE=1 go test -run TestGoldenStrace.

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

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// canonDump is the same canonical rendering the parser package's golden test
// uses (it cannot be imported: it lives in a _test file, and this package is
// a dependent of that one). See parser/golden_test.go for the format.
func canonDump(w io.Writer, n *parser.ASTNode) (nodes, toks int) {
	seen := map[*interface{}]int{}
	type frame struct {
		n    *parser.ASTNode
		next int
	}
	var stack []frame
	enter := func(n *parser.ASTNode) {
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
		case *parser.ASTNode:
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

var goldenOptions = []struct {
	name string
	o    Options
}{
	{"default", Options{}},
	{"depth5", Options{Options: parser.Options{MaxDepth: 5}}},
	{"depth9", Options{Options: parser.Options{MaxDepth: 9}}},
	{"memo40", Options{Options: parser.Options{MaxMemoSlots: 40}}},
	{"memo300", Options{Options: parser.Options{MaxMemoSlots: 300}}},
	{"tokens7", Options{Options: parser.Options{MaxTokens: 7}}},
}

func straceGoldenLine(line string, opt string, o Options) (key, rec string) {
	sum := sha256.Sum256([]byte(line))
	key = hex.EncodeToString(sum[:6]) + "/" + opt
	ast, toks, err := ParseLineTokens(line, o)
	h := sha256.New()
	var summary string
	if err != nil {
		var le *LineError
		kind := -1
		var tok lexer.Token
		if errors.As(err, &le) {
			kind, tok = int(le.Kind), le.Token
		}
		fmt.Fprintf(h, "ERR nil=%v toks=%d kind=%d msg=%q tok=%s:%q:%d:%d", ast == nil, len(toks), kind, err.Error(),
			tok.TypeName, tok.Value, tok.Line, tok.Column)
		summary = fmt.Sprintf("err kind=%d %q", kind, err.Error())
	} else {
		nodes, nt := canonDump(h, ast)
		fmt.Fprintf(h, " toks=%d", len(toks))
		summary = fmt.Sprintf("ok nodes=%d tokens=%d", nodes, nt)
	}
	return key, key + "\t" + hex.EncodeToString(h.Sum(nil)) + "\t" + summary
}

func TestGoldenStrace(t *testing.T) {
	// Inputs: the corpus (which includes every refused line), plus the
	// committed .strace fixtures. De-duplicated, in a stable order.
	seen := map[string]bool{}
	var inputs []string
	add := func(path string) {
		for _, l := range lines(t, path) {
			if !seen[l] {
				seen[l] = true
				inputs = append(inputs, l)
			}
		}
	}
	add("testdata/corpus.txt")
	add("testdata/corpus_refused.txt")
	fixtures, _ := filepath.Glob("testdata/*.strace")
	sort.Strings(fixtures)
	for _, f := range fixtures {
		add(f)
	}

	got := map[string]string{}
	var order []string
	for _, l := range inputs {
		for _, g := range goldenOptions {
			key, rec := straceGoldenLine(l, g.name, g.o)
			if _, dup := got[key]; !dup {
				order = append(order, key)
			}
			got[key] = rec
		}
	}

	path := filepath.Join("testdata", "golden", "strace.golden")
	if os.Getenv("GOLDEN_UPDATE") != "" {
		sort.Strings(order)
		var sb strings.Builder
		for _, k := range order {
			sb.WriteString(got[k])
			sb.WriteByte('\n')
		}
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(sb.String()), 0o644); err != nil {
			t.Fatal(err)
		}
		return
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("%v (generate with GOLDEN_UPDATE=1)", err)
	}
	matched := 0
	for _, l := range strings.Split(strings.TrimSuffix(string(raw), "\n"), "\n") {
		key, _, _ := strings.Cut(l, "\t")
		have, ok := got[key]
		if !ok {
			continue // the corpus line was removed
		}
		if have != l {
			t.Errorf("golden mismatch\n want %s\n  got %s", l, have)
			continue
		}
		matched++
	}
	const minMatched = 1500
	if matched < minMatched {
		t.Errorf("only %d golden cases matched (want at least %d): regenerate the goldens", matched, minMatched)
	}
}
