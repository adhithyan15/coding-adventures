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
func TestParseCrossCheckedOnlyInCensus(t *testing.T) {
	fset := token.NewFileSet()
	err := filepath.WalkDir(".", func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
			return nil
		}
		if slash := filepath.ToSlash(path); slash == "internal/trace/crosscheck.go" || slash == "internal/record/census_report.go" {
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
				if x.Sel.Name == "ParseCrossChecked" {
					t.Errorf("%s: references ParseCrossChecked outside `kiln record census`", fset.Position(x.Pos()))
				}
			case *ast.Ident:
				if x.Name == "ParseCrossChecked" {
					t.Errorf("%s: mentions ParseCrossChecked outside `kiln record census`", fset.Position(x.Pos()))
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
