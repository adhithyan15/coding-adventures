package main

import (
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
// a recording) may call it; no other non-test file may.  Its definition file is
// the other exception.
func TestParseCrossCheckedOnlyInCensus(t *testing.T) {
	re := regexp.MustCompile(`\bParseCrossChecked\(`)
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
		src, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		for i, line := range strings.Split(string(src), "\n") {
			if trimmed := strings.TrimSpace(line); strings.HasPrefix(trimmed, "//") {
				continue
			}
			if re.MatchString(line) {
				t.Errorf("%s:%d calls ParseCrossChecked outside `kiln record census`: %s", path, i+1, strings.TrimSpace(line))
			}
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}
