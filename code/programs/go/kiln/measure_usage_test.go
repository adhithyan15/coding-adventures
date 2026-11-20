package main

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// TestMeasureIsNotUsedByProductionCode: trace.Measure, MeasureBest, BestPass1,
// BestPass1Calibrated, RefBytes and Diagnose are CI/test instruments.  For a small trace they switch the garbage collector
// off with debug.SetGCPercent, which is process-global and not goroutine-safe
// (internal/trace/measure.go), so no non-test source file under kiln may call
// them.  Their own definition file (measure.go) is the only exception.  (D34's
// ParseCrossChecked does not touch the GC; it has its own guard below.)
func TestMeasureIsNotUsedByProductionCode(t *testing.T) {
	re := regexp.MustCompile(`\b(Measure|MeasureBest|BestPass1|BestPass1Calibrated|RefBytes|Diagnose)\(`)
	err := filepath.WalkDir(".", func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
			return nil
		}
		if slash := filepath.ToSlash(path); slash == "internal/trace/measure.go" {
			return nil
		}
		src, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		for i, line := range strings.Split(string(src), "\n") {
			if trimmed := strings.TrimSpace(line); strings.HasPrefix(trimmed, "//") {
				continue
			}
			if re.MatchString(line) {
				t.Errorf("%s:%d calls a CI-only measurement function: %s", path, i+1, strings.TrimSpace(line))
			}
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}

// TestParseCrossCheckedOnlyInCensus: trace.ParseCrossChecked is serialised and
// does not toggle the GC, so `kiln record census` (a diagnostic never run during
// a recording) may use it; no other non-test file may.  The check is on the
// syntax tree, so it catches ANY reference (a call, a method value, a function
// stored in a variable, an alias import, a dot import), not just a textual
// "ParseCrossChecked(".  Its definition file and census_report.go are the only
// exceptions.
//
// The same guard covers ParseCrossCheckedDetail (the census's variant that names
// each allowed divergence's row) and trace.Shape, the REDACTED line outline that
// is safe only because the census is its one caller (internal/trace/shape.go):
// it prints no byte of a token's text, but it reads real trace lines, so no
// recording path may reach it, and its identifiers (Shape, LineShape) are
// flagged anywhere else.
var censusOnly = map[string]bool{
	"ParseCrossChecked": true, "ParseCrossCheckedDetail": true, "Shape": true, "LineShape": true,
}

func TestParseCrossCheckedOnlyInCensus(t *testing.T) {
	fset := token.NewFileSet()
	err := filepath.WalkDir(".", func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
			return nil
		}
		if slash := filepath.ToSlash(path); slash == "internal/trace/crosscheck.go" || slash == "internal/trace/shape.go" || slash == "internal/record/census_report.go" {
			return nil
		}
		f, err := parser.ParseFile(fset, path, nil, 0)
		if err != nil {
			return err
		}
		for _, imp := range f.Imports {
			if imp.Name != nil && imp.Name.Name == "." && strings.HasSuffix(strings.Trim(imp.Path.Value, `"`), "/internal/trace") {
				t.Errorf("%s dot-imports the trace package, which hides references to ParseCrossChecked", path)
			}
		}
		ast.Inspect(f, func(n ast.Node) bool {
			switch x := n.(type) {
			case *ast.SelectorExpr:
				if censusOnly[x.Sel.Name] {
					t.Errorf("%s: references %s outside `kiln record census`", fset.Position(x.Pos()), x.Sel.Name)
				}
			case *ast.Ident:
				if censusOnly[x.Name] {
					t.Errorf("%s: mentions %s outside `kiln record census`", fset.Position(x.Pos()), x.Name)
				}
			}
			return true
		})
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}
