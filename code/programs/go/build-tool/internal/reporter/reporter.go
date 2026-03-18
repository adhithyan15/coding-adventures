// Package reporter formats and prints a summary table of build results.
//
// # Output format
//
// The report is designed for terminal display — a fixed-width table with
// aligned columns, followed by a summary line:
//
//	Build Report
//	============
//	Package                    Status     Duration
//	python/logic-gates         SKIPPED    -
//	python/arithmetic          BUILT      2.3s
//	python/arm-simulator       FAILED     0.5s
//	python/riscv-simulator     DEP-SKIP   - (dep failed)
//
//	Total: 21 packages | 5 built | 14 skipped | 1 failed | 1 dep-skipped
//
// The report is sorted by package name for consistent output across runs.
// Status names are uppercased for visual prominence.
package reporter

import (
	"fmt"
	"io"
	"os"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
)

// statusDisplay maps internal status strings to display names.
// We uppercase for visual clarity in the terminal.
var statusDisplay = map[string]string{
	"built":       "BUILT",
	"failed":      "FAILED",
	"skipped":     "SKIPPED",
	"dep-skipped": "DEP-SKIP",
	"would-build": "WOULD-BUILD",
}

// formatDuration converts seconds to a display string.
// Returns "-" for negligible durations, otherwise "X.Ys".
func formatDuration(seconds float64) string {
	if seconds < 0.01 {
		return "-"
	}
	return fmt.Sprintf("%.1fs", seconds)
}

// FormatReport produces the build report as a string. This is the pure
// function — it doesn't print anything, making it easy to test.
func FormatReport(results map[string]executor.BuildResult) string {
	var buf strings.Builder

	buf.WriteString("\nBuild Report\n")
	buf.WriteString("============\n")

	if len(results) == 0 {
		buf.WriteString("No packages processed.\n")
		return buf.String()
	}

	// Calculate the maximum package name length for column alignment.
	maxNameLen := len("Package")
	for name := range results {
		if len(name) > maxNameLen {
			maxNameLen = len(name)
		}
	}

	// Header row.
	buf.WriteString(fmt.Sprintf("%-*s   %-12s %s\n", maxNameLen, "Package", "Status", "Duration"))

	// Sort results by package name for consistent output.
	names := make([]string, 0, len(results))
	for name := range results {
		names = append(names, name)
	}
	sort.Strings(names)

	// Data rows.
	for _, name := range names {
		result := results[name]
		status := statusDisplay[result.Status]
		if status == "" {
			status = strings.ToUpper(result.Status)
		}
		duration := formatDuration(result.Duration)
		if result.Status == "dep-skipped" {
			duration = "- (dep failed)"
		}
		buf.WriteString(fmt.Sprintf("%-*s   %-12s %s\n", maxNameLen, name, status, duration))
	}

	// Summary line — counts of each status.
	total := len(results)
	built := 0
	skipped := 0
	failed := 0
	depSkipped := 0
	wouldBuild := 0
	for _, r := range results {
		switch r.Status {
		case "built":
			built++
		case "skipped":
			skipped++
		case "failed":
			failed++
		case "dep-skipped":
			depSkipped++
		case "would-build":
			wouldBuild++
		}
	}

	buf.WriteString(fmt.Sprintf("\nTotal: %d packages", total))
	if built > 0 {
		buf.WriteString(fmt.Sprintf(" | %d built", built))
	}
	if skipped > 0 {
		buf.WriteString(fmt.Sprintf(" | %d skipped", skipped))
	}
	if failed > 0 {
		buf.WriteString(fmt.Sprintf(" | %d failed", failed))
	}
	if depSkipped > 0 {
		buf.WriteString(fmt.Sprintf(" | %d dep-skipped", depSkipped))
	}
	if wouldBuild > 0 {
		buf.WriteString(fmt.Sprintf(" | %d would-build", wouldBuild))
	}
	buf.WriteString("\n")

	return buf.String()
}

// PrintReport prints the build report to the given writer (or stdout).
func PrintReport(results map[string]executor.BuildResult, w io.Writer) {
	if w == nil {
		w = os.Stdout
	}
	report := FormatReport(results)
	fmt.Fprint(w, report)
}
