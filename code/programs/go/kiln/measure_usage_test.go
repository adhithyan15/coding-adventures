package main

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// TestMeasureIsNotUsedByProductionCode: trace.Measure, MeasureBest, BestPass1,
// BestPass1Calibrated, RefBytes, Diagnose and ParseCrossChecked
// are CI/test instruments.  For a small trace they switch the garbage collector
// off with debug.SetGCPercent, which is process-global and not goroutine-safe
// (internal/trace/measure.go), so no non-test source file under kiln may call
// them.  Their own definition files (measure.go; crosscheck.go for
// ParseCrossChecked, D34) are the only exceptions.
func TestMeasureIsNotUsedByProductionCode(t *testing.T) {
	re := regexp.MustCompile(`\b(Measure|MeasureBest|BestPass1|BestPass1Calibrated|RefBytes|Diagnose|ParseCrossChecked)\(`)
	err := filepath.WalkDir(".", func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
			return nil
		}
		if slash := filepath.ToSlash(path); slash == "internal/trace/measure.go" || slash == "internal/trace/crosscheck.go" {
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
