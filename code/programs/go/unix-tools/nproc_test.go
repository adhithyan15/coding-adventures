// =========================================================================
// nproc — Tests
// =========================================================================
//
// These tests verify the nproc tool's behavior:
//
//   1. Default: prints the number of available CPUs
//   2. --ignore N: subtracts N from the count (minimum 1)
//   3. calculateNproc: unit tests for the calculation logic
//   4. --help and --version: standard meta-flags
//   5. Error handling: invalid spec path returns exit code 1

package main

import (
	"bytes"
	"fmt"
	"runtime"
	"strconv"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestNprocSpecLoads verifies that nproc.json is a valid cli-builder spec.
func TestNprocSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "nproc"), []string{"nproc"})
	if err != nil {
		t.Fatalf("failed to load nproc.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// calculateNproc unit tests
// =========================================================================

// TestCalculateNprocNoIgnore verifies basic CPU count without subtraction.
func TestCalculateNprocNoIgnore(t *testing.T) {
	tests := []struct {
		total    int
		ignore   int
		expected int
	}{
		{8, 0, 8},
		{4, 0, 4},
		{1, 0, 1},
		{16, 0, 16},
	}

	for _, tt := range tests {
		result := calculateNproc(tt.total, tt.ignore)
		if result != tt.expected {
			t.Errorf("calculateNproc(%d, %d) = %d, want %d", tt.total, tt.ignore, result, tt.expected)
		}
	}
}

// TestCalculateNprocWithIgnore verifies subtraction of the ignore value.
func TestCalculateNprocWithIgnore(t *testing.T) {
	tests := []struct {
		total    int
		ignore   int
		expected int
	}{
		{8, 2, 6},
		{8, 4, 4},
		{8, 7, 1},
		{4, 1, 3},
		{16, 8, 8},
	}

	for _, tt := range tests {
		result := calculateNproc(tt.total, tt.ignore)
		if result != tt.expected {
			t.Errorf("calculateNproc(%d, %d) = %d, want %d", tt.total, tt.ignore, result, tt.expected)
		}
	}
}

// TestCalculateNprocClampsToOne verifies that the result never goes below 1.
func TestCalculateNprocClampsToOne(t *testing.T) {
	tests := []struct {
		total  int
		ignore int
	}{
		{8, 8},   // exactly equal
		{8, 10},  // more than total
		{4, 100}, // way more than total
		{1, 1},   // single CPU, ignore 1
		{1, 5},   // single CPU, ignore many
	}

	for _, tt := range tests {
		result := calculateNproc(tt.total, tt.ignore)
		if result != 1 {
			t.Errorf("calculateNproc(%d, %d) = %d, want 1 (minimum clamp)", tt.total, tt.ignore, result)
		}
	}
}

// =========================================================================
// runNproc integration tests
// =========================================================================

// TestRunNprocDefault verifies that nproc prints the CPU count.
func TestRunNprocDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNproc(toolSpecPath(t, "nproc"), []string{"nproc"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runNproc() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	count, err := strconv.Atoi(output)
	if err != nil {
		t.Fatalf("runNproc() output %q is not an integer: %v", output, err)
	}

	expected := runtime.NumCPU()
	if count != expected {
		t.Errorf("runNproc() = %d, want %d (runtime.NumCPU())", count, expected)
	}
}

// TestRunNprocAll verifies that --all prints the CPU count (same as default in Go).
func TestRunNprocAll(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNproc(toolSpecPath(t, "nproc"), []string{"nproc", "--all"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runNproc(--all) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	count, err := strconv.Atoi(output)
	if err != nil {
		t.Fatalf("runNproc(--all) output %q is not an integer: %v", output, err)
	}

	// In Go, --all gives the same result as the default.
	if count < 1 {
		t.Errorf("runNproc(--all) = %d, want >= 1", count)
	}
}

// TestRunNprocIgnore verifies that --ignore subtracts from the CPU count.
func TestRunNprocIgnore(t *testing.T) {
	totalCPUs := runtime.NumCPU()

	// Only test --ignore if we have more than 1 CPU, otherwise the result
	// is always 1 regardless of --ignore.
	if totalCPUs <= 1 {
		t.Skip("only 1 CPU available, --ignore test would be trivial")
	}

	var stdout, stderr bytes.Buffer
	code := runNproc(toolSpecPath(t, "nproc"), []string{"nproc", "--ignore", "1"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runNproc(--ignore 1) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	count, err := strconv.Atoi(output)
	if err != nil {
		t.Fatalf("runNproc(--ignore 1) output %q is not an integer: %v", output, err)
	}

	expected := totalCPUs - 1
	if count != expected {
		t.Errorf("runNproc(--ignore 1) = %d, want %d", count, expected)
	}
}

// TestRunNprocIgnoreTooMany verifies that --ignore with a value >= total CPUs
// produces 1 (minimum clamp).
func TestRunNprocIgnoreTooMany(t *testing.T) {
	totalCPUs := runtime.NumCPU()

	var stdout, stderr bytes.Buffer
	ignoreStr := fmt.Sprintf("%d", totalCPUs+10)
	code := runNproc(toolSpecPath(t, "nproc"), []string{"nproc", "--ignore", ignoreStr}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runNproc(--ignore too_many) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1" {
		t.Errorf("runNproc(--ignore too_many) = %q, want %q", output, "1")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestNprocHelpFlag verifies that --help prints help text and returns 0.
func TestNprocHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNproc(toolSpecPath(t, "nproc"), []string{"nproc", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runNproc(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runNproc(--help) produced no stdout output")
	}
}

// TestNprocVersionFlag verifies that --version prints the version.
func TestNprocVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNproc(toolSpecPath(t, "nproc"), []string{"nproc", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runNproc(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runNproc(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestNprocInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestNprocInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNproc("/nonexistent/nproc.json", []string{"nproc"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runNproc(bad spec) returned exit code %d, want 1", code)
	}
}
