package traced

import (
	"bytes"
	"encoding/json"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The invariant the comment at MaxCensusEntries states: every kind the
// classifier can feed has a per-kind cap, and the caps add up to at most the
// overall bound, so the overall bound never decides which kind loses.
func TestCensusKindCapsFitTheOverallBound(t *testing.T) {
	// noteReason files each censusPrefixes entry under its own kind (host-read
	// and its two refinements included): eight kinds, plus proc-path.
	distinct := map[string]bool{}
	for _, p := range censusPrefixes {
		distinct[strings.TrimSuffix(p, ":")] = true
	}
	if len(distinct) != 8 {
		t.Fatalf("%d censusPrefixes kinds, the cap arithmetic assumes 8", len(distinct))
	}
	distinct["trace-incomplete:proc-path"] = true
	total := 0
	for k := range distinct {
		total += kindCap(k)
	}
	for reason, subs := range censusSubCodes {
		for _, s := range subs {
			total += kindCap(reason + ":" + s)
		}
	}
	if total > MaxCensusEntries {
		t.Errorf("the per-kind caps add up to %d, above MaxCensusEntries %d: a late kind can be starved", total, MaxCensusEntries)
	}
	if kindCap("trace-incomplete:unstable-path:dotdot") != MaxCensusPerSubKind || kindCap("host-read") != MaxCensusPerKind {
		t.Error("kindCap")
	}
	// A sub-coded kind holds at most its cap.
	c := NewCensus()
	for i := 0; i < MaxCensusPerSubKind+50; i++ {
		c.noteClean("trace-incomplete:unstable-path:dotdot", fmt.Sprintf("/p/%d", i))
	}
	if n := len(censusKinds(c)["trace-incomplete:unstable-path:dotdot"]); n != MaxCensusPerSubKind || c.Overflow["trace-incomplete:unstable-path:dotdot"] != 50 {
		t.Errorf("held %d, overflow %v", n, c.Overflow)
	}
}

// Every sub-code the classifier passes (a string literal at the call site, or
// the literal after rUnstable/rExecUnreadable in a note) is on the closed list
// the cap arithmetic above counts.
func TestCensusSubCodesAreTheClosedList(t *testing.T) {
	known := map[string]bool{}
	for _, subs := range censusSubCodes {
		for _, s := range subs {
			known[s] = true
		}
	}
	files, _ := filepath.Glob("*.go")
	fset := token.NewFileSet()
	found := 0
	for _, f := range files {
		if strings.HasSuffix(f, "_test.go") {
			continue
		}
		file, err := parser.ParseFile(fset, f, nil, 0)
		if err != nil {
			t.Fatal(err)
		}
		ast.Inspect(file, func(n ast.Node) bool {
			call, ok := n.(*ast.CallExpr)
			if !ok {
				return true
			}
			var lit ast.Expr
			switch fn := call.Fun.(type) {
			case *ast.SelectorExpr:
				switch fn.Sel.Name {
				case "reasonSub":
					lit = call.Args[1]
				case "add": // notes.add(census, rUnstable+":sub", path)
					if len(call.Args) == 3 {
						if b, ok := call.Args[1].(*ast.BinaryExpr); ok {
							lit = b.Y
						}
					}
				}
			}
			bl, ok := lit.(*ast.BasicLit)
			if !ok {
				return true
			}
			s, _ := strconv.Unquote(bl.Value)
			s = strings.TrimPrefix(s, ":")
			found++
			if !known[s] {
				t.Errorf("%s: sub-code %q is not in censusSubCodes", fset.Position(bl.Pos()), s)
			}
			return true
		})
	}
	if found < 6 {
		t.Errorf("found only %d sub-code literals: the scan is broken", found)
	}
	// headWhere returns literals too.
	for _, s := range append([]string{"under-tmp", "in-repo", "host-disk"}, execHeadSubCodes...) {
		if !known[s] {
			t.Errorf("headWhere's %q is not in censusSubCodes", s)
		}
	}
}

// Two classifications of the same events give the same census.json, even when
// a kind overflows its cap: judge walks maps, and which paths a full kind keeps
// must not depend on their order.
func TestCensusIsDeterministicAtTheCap(t *testing.T) {
	var evs []trace.Event
	const n = MaxCensusPerSubKind + 60
	for i := 0; i < n; i++ {
		evs = append(evs,
			symlink("../b", fmt.Sprintf("code/p/a/t%03d", i)),
			rename(fmt.Sprintf("code/p/a/t%03d", i), fmt.Sprintf("code/p/a/f%03d", i)),
			stat(fmt.Sprintf("code/p/a/f%03d/b.txt", i)),
		)
	}
	var first []byte
	for run := 0; run < 6; run++ {
		h := newHarness(t)
		for i := 0; i < n; i++ {
			h.fs.link(abs(fmt.Sprintf("code/p/a/f%03d", i)), "../b")
		}
		_, c := h.runCensus(evs...)
		over := int64(0)
		for k, v := range c.Overflow {
			if strings.HasPrefix(k, "trace-incomplete:unstable-path:") {
				over += v
			}
		}
		if over == 0 {
			t.Fatalf("the scenario did not overflow a sub-coded kind: %v", c.Overflow)
		}
		data, err := json.Marshal(struct {
			E []CensusEntry
			O map[string]int64
			T int64
		}{c.Entries(), c.Overflow, c.Total})
		if err != nil {
			t.Fatal(err)
		}
		if first == nil {
			first = data
		} else if !bytes.Equal(first, data) {
			t.Fatalf("run %d differs from run 0 (%d and %d bytes of census)", run, len(first), len(data))
		}
	}
}
