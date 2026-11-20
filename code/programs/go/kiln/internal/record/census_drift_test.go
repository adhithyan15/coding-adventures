package record

// The census's closed lists (census_explain.go) must not fall behind the code
// that produces the reasons: a new reason constant that the lists do not know
// would silently be counted as "other".  Both packages keep their constants
// unexported, so this test reads their source with go/parser.

import (
	"go/ast"
	"go/parser"
	"go/token"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"testing"
)

// stringConsts returns the string constants of the non-test files of dir whose
// name starts with prefix followed by an upper-case letter.
func stringConsts(t *testing.T, dir, prefix string) map[string]string {
	t.Helper()
	files, err := filepath.Glob(filepath.Join(dir, "*.go"))
	if err != nil || len(files) == 0 {
		t.Fatalf("no sources in %s: %v", dir, err)
	}
	out := map[string]string{}
	fset := token.NewFileSet()
	for _, f := range files {
		if strings.HasSuffix(f, "_test.go") {
			continue
		}
		file, err := parser.ParseFile(fset, f, nil, 0)
		if err != nil {
			t.Fatal(err)
		}
		for _, d := range file.Decls {
			gd, ok := d.(*ast.GenDecl)
			if !ok || gd.Tok != token.CONST {
				continue
			}
			for _, sp := range gd.Specs {
				vs := sp.(*ast.ValueSpec)
				for i, n := range vs.Names {
					if len(n.Name) <= len(prefix) || !strings.HasPrefix(n.Name, prefix) || n.Name[len(prefix)] < 'A' || n.Name[len(prefix)] > 'Z' || i >= len(vs.Values) {
						continue
					}
					if lit, ok := vs.Values[i].(*ast.BasicLit); ok && lit.Kind == token.STRING {
						v, err := strconv.Unquote(lit.Value)
						if err != nil {
							t.Fatal(err)
						}
						out[n.Name] = v
					}
				}
			}
		}
	}
	return out
}

func sortedConstNames(m map[string]string) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	sort.Strings(ks)
	return ks
}

// Every classifier reason constant (internal/traced, rXxx) maps to itself, not
// to "other" or a shorter kind.
func TestReasonCodesCoverEveryClassifierReason(t *testing.T) {
	consts := stringConsts(t, "../traced", "r")
	if len(consts) < 15 {
		t.Fatalf("found only %d reason constants: the scan is broken", len(consts))
	}
	for _, name := range sortedConstNames(consts) {
		v := consts[name]
		if got := reasonKindCode(v); got != v {
			t.Errorf("traced.%s = %q maps to %q: add it to reasonCodes", name, v, got)
		}
	}
}

// Every trace-error reason constant (internal/trace, reasonXxx) maps to itself
// through parseReasonCode, bare and as the "trace-incomplete:" text of an error.
func TestTraceReasonCodesCoverEveryParserReason(t *testing.T) {
	consts := stringConsts(t, "../trace", "reason")
	if len(consts) < 30 {
		t.Fatalf("found only %d reason constants: the scan is broken", len(consts))
	}
	for _, name := range sortedConstNames(consts) {
		v := consts[name]
		for _, in := range []string{v, "trace-incomplete:" + v, "step 0: trace-incomplete:" + v + ":x"} {
			if got := parseReasonCode(in); got != v {
				t.Errorf("trace.%s = %q: parseReasonCode(%q) = %q: add it to traceReasonCodes", name, v, in, got)
			}
		}
	}
}
