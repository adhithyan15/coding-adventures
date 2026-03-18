package reporter

import (
	"bytes"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
)

// ---------------------------------------------------------------------------
// Tests for formatDuration
// ---------------------------------------------------------------------------

func TestFormatDurationZero(t *testing.T) {
	if formatDuration(0.0) != "-" {
		t.Fatalf("expected '-', got %s", formatDuration(0.0))
	}
}

func TestFormatDurationSmall(t *testing.T) {
	if formatDuration(0.001) != "-" {
		t.Fatalf("expected '-' for small duration, got %s", formatDuration(0.001))
	}
}

func TestFormatDurationNormal(t *testing.T) {
	result := formatDuration(2.345)
	if result != "2.3s" {
		t.Fatalf("expected 2.3s, got %s", result)
	}
}

func TestFormatDurationLarge(t *testing.T) {
	result := formatDuration(123.456)
	if result != "123.5s" {
		t.Fatalf("expected 123.5s, got %s", result)
	}
}

// ---------------------------------------------------------------------------
// Tests for FormatReport
// ---------------------------------------------------------------------------

func TestFormatReportEmpty(t *testing.T) {
	report := FormatReport(map[string]executor.BuildResult{})
	if !strings.Contains(report, "No packages processed") {
		t.Fatal("expected 'No packages processed' in empty report")
	}
}

func TestFormatReportSingleBuilt(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/pkg-a": {PackageName: "python/pkg-a", Status: "built", Duration: 1.5},
	}
	report := FormatReport(results)

	if !strings.Contains(report, "BUILT") {
		t.Fatal("expected BUILT in report")
	}
	if !strings.Contains(report, "1.5s") {
		t.Fatal("expected 1.5s in report")
	}
	if !strings.Contains(report, "1 built") {
		t.Fatal("expected '1 built' in summary")
	}
}

func TestFormatReportMixed(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/pkg-a": {PackageName: "python/pkg-a", Status: "built", Duration: 2.0},
		"python/pkg-b": {PackageName: "python/pkg-b", Status: "skipped"},
		"python/pkg-c": {PackageName: "python/pkg-c", Status: "failed", Duration: 0.5},
		"python/pkg-d": {PackageName: "python/pkg-d", Status: "dep-skipped"},
	}
	report := FormatReport(results)

	// Check all statuses appear.
	for _, expected := range []string{"BUILT", "SKIPPED", "FAILED", "DEP-SKIP"} {
		if !strings.Contains(report, expected) {
			t.Errorf("expected %s in report", expected)
		}
	}

	// Check summary.
	if !strings.Contains(report, "Total: 4 packages") {
		t.Error("expected 'Total: 4 packages' in summary")
	}
	if !strings.Contains(report, "1 built") {
		t.Error("expected '1 built' in summary")
	}
	if !strings.Contains(report, "1 skipped") {
		t.Error("expected '1 skipped' in summary")
	}
	if !strings.Contains(report, "1 failed") {
		t.Error("expected '1 failed' in summary")
	}
	if !strings.Contains(report, "1 dep-skipped") {
		t.Error("expected '1 dep-skipped' in summary")
	}
}

func TestFormatReportDepSkippedDuration(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/pkg-a": {PackageName: "python/pkg-a", Status: "dep-skipped"},
	}
	report := FormatReport(results)

	if !strings.Contains(report, "- (dep failed)") {
		t.Fatal("dep-skipped should show '- (dep failed)' for duration")
	}
}

func TestFormatReportSortedByName(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/zz-pkg": {PackageName: "python/zz-pkg", Status: "built"},
		"python/aa-pkg": {PackageName: "python/aa-pkg", Status: "built"},
		"python/mm-pkg": {PackageName: "python/mm-pkg", Status: "built"},
	}
	report := FormatReport(results)

	// aa should appear before mm, mm before zz.
	aaIdx := strings.Index(report, "python/aa-pkg")
	mmIdx := strings.Index(report, "python/mm-pkg")
	zzIdx := strings.Index(report, "python/zz-pkg")

	if aaIdx > mmIdx || mmIdx > zzIdx {
		t.Fatal("report should be sorted by package name")
	}
}

func TestFormatReportWouldBuild(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/pkg-a": {PackageName: "python/pkg-a", Status: "would-build"},
	}
	report := FormatReport(results)

	if !strings.Contains(report, "WOULD-BUILD") {
		t.Fatal("expected WOULD-BUILD in report")
	}
	if !strings.Contains(report, "1 would-build") {
		t.Fatal("expected '1 would-build' in summary")
	}
}

// ---------------------------------------------------------------------------
// Tests for PrintReport
// ---------------------------------------------------------------------------

func TestPrintReportToWriter(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/pkg-a": {PackageName: "python/pkg-a", Status: "built", Duration: 1.0},
	}

	var buf bytes.Buffer
	PrintReport(results, &buf)

	output := buf.String()
	if !strings.Contains(output, "Build Report") {
		t.Fatal("expected 'Build Report' in output")
	}
}

func TestFormatReportHeader(t *testing.T) {
	results := map[string]executor.BuildResult{
		"python/pkg-a": {PackageName: "python/pkg-a", Status: "built"},
	}
	report := FormatReport(results)

	if !strings.Contains(report, "Build Report") {
		t.Fatal("expected 'Build Report' header")
	}
	if !strings.Contains(report, "============") {
		t.Fatal("expected '============' separator")
	}
	if !strings.Contains(report, "Package") {
		t.Fatal("expected 'Package' column header")
	}
	if !strings.Contains(report, "Status") {
		t.Fatal("expected 'Status' column header")
	}
	if !strings.Contains(report, "Duration") {
		t.Fatal("expected 'Duration' column header")
	}
}
