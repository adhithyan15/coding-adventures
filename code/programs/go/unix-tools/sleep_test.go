// =========================================================================
// sleep — Tests
// =========================================================================
//
// These tests verify the sleep tool's behavior:
//
//   1. parseDuration: unit tests for duration string parsing
//   2. Duration suffixes: s, m, h, d
//   3. Fractional values: 2.5s, 0.1m
//   4. Multiple durations: summing
//   5. Error handling: invalid durations, empty strings
//   6. --help and --version: standard meta-flags
//
// We focus on testing parseDuration() directly since time.Sleep() is
// not practical to test (we don't want tests that actually sleep).

package main

import (
	"bytes"
	"math"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestSleepSpecLoads verifies that sleep.json is a valid cli-builder spec.
func TestSleepSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "sleep"), []string{"sleep", "1"})
	if err != nil {
		t.Fatalf("failed to load sleep.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// parseDuration unit tests — basic cases
// =========================================================================

// TestParseDurationPlainNumber verifies that a plain number is parsed as seconds.
func TestParseDurationPlainNumber(t *testing.T) {
	val, err := parseDuration("5")
	if err != nil {
		t.Fatalf("parseDuration(\"5\") failed: %v", err)
	}
	if val != 5.0 {
		t.Errorf("parseDuration(\"5\") = %f, want 5.0", val)
	}
}

// TestParseDurationZero verifies that 0 is valid.
func TestParseDurationZero(t *testing.T) {
	val, err := parseDuration("0")
	if err != nil {
		t.Fatalf("parseDuration(\"0\") failed: %v", err)
	}
	if val != 0.0 {
		t.Errorf("parseDuration(\"0\") = %f, want 0.0", val)
	}
}

// TestParseDurationFractional verifies that fractional seconds work.
func TestParseDurationFractional(t *testing.T) {
	val, err := parseDuration("2.5")
	if err != nil {
		t.Fatalf("parseDuration(\"2.5\") failed: %v", err)
	}
	if val != 2.5 {
		t.Errorf("parseDuration(\"2.5\") = %f, want 2.5", val)
	}
}

// =========================================================================
// parseDuration unit tests — suffixes
// =========================================================================

// TestParseDurationSeconds verifies the 's' suffix.
func TestParseDurationSeconds(t *testing.T) {
	val, err := parseDuration("10s")
	if err != nil {
		t.Fatalf("parseDuration(\"10s\") failed: %v", err)
	}
	if val != 10.0 {
		t.Errorf("parseDuration(\"10s\") = %f, want 10.0", val)
	}
}

// TestParseDurationMinutes verifies the 'm' suffix.
func TestParseDurationMinutes(t *testing.T) {
	val, err := parseDuration("2m")
	if err != nil {
		t.Fatalf("parseDuration(\"2m\") failed: %v", err)
	}
	if val != 120.0 {
		t.Errorf("parseDuration(\"2m\") = %f, want 120.0", val)
	}
}

// TestParseDurationHours verifies the 'h' suffix.
func TestParseDurationHours(t *testing.T) {
	val, err := parseDuration("1h")
	if err != nil {
		t.Fatalf("parseDuration(\"1h\") failed: %v", err)
	}
	if val != 3600.0 {
		t.Errorf("parseDuration(\"1h\") = %f, want 3600.0", val)
	}
}

// TestParseDurationDays verifies the 'd' suffix.
func TestParseDurationDays(t *testing.T) {
	val, err := parseDuration("1d")
	if err != nil {
		t.Fatalf("parseDuration(\"1d\") failed: %v", err)
	}
	if val != 86400.0 {
		t.Errorf("parseDuration(\"1d\") = %f, want 86400.0", val)
	}
}

// TestParseDurationFractionalMinutes verifies fractional minutes.
func TestParseDurationFractionalMinutes(t *testing.T) {
	val, err := parseDuration("0.5m")
	if err != nil {
		t.Fatalf("parseDuration(\"0.5m\") failed: %v", err)
	}
	if val != 30.0 {
		t.Errorf("parseDuration(\"0.5m\") = %f, want 30.0", val)
	}
}

// TestParseDurationFractionalHours verifies fractional hours.
func TestParseDurationFractionalHours(t *testing.T) {
	val, err := parseDuration("1.5h")
	if err != nil {
		t.Fatalf("parseDuration(\"1.5h\") failed: %v", err)
	}
	if val != 5400.0 {
		t.Errorf("parseDuration(\"1.5h\") = %f, want 5400.0", val)
	}
}

// =========================================================================
// parseDuration unit tests — table-driven
// =========================================================================

// TestParseDurationTable runs a comprehensive table of test cases.
func TestParseDurationTable(t *testing.T) {
	tests := []struct {
		input    string
		expected float64
	}{
		{"0", 0},
		{"1", 1},
		{"100", 100},
		{"0.1", 0.1},
		{"5s", 5},
		{"5m", 300},
		{"5h", 18000},
		{"5d", 432000},
		{"0s", 0},
		{"0m", 0},
		{"0h", 0},
		{"0d", 0},
		{"0.001s", 0.001},
	}

	for _, tt := range tests {
		val, err := parseDuration(tt.input)
		if err != nil {
			t.Errorf("parseDuration(%q) failed: %v", tt.input, err)
			continue
		}
		if math.Abs(val-tt.expected) > 0.0001 {
			t.Errorf("parseDuration(%q) = %f, want %f", tt.input, val, tt.expected)
		}
	}
}

// =========================================================================
// parseDuration unit tests — error cases
// =========================================================================

// TestParseDurationEmpty verifies that empty string returns an error.
func TestParseDurationEmpty(t *testing.T) {
	_, err := parseDuration("")
	if err == nil {
		t.Error("parseDuration(\"\") should return error")
	}
}

// TestParseDurationInvalidText verifies that non-numeric text returns an error.
func TestParseDurationInvalidText(t *testing.T) {
	_, err := parseDuration("abc")
	if err == nil {
		t.Error("parseDuration(\"abc\") should return error")
	}
}

// TestParseDurationNegative verifies that negative durations return an error.
func TestParseDurationNegative(t *testing.T) {
	_, err := parseDuration("-5")
	if err == nil {
		t.Error("parseDuration(\"-5\") should return error")
	}
}

// TestParseDurationNegativeWithSuffix verifies negative duration with suffix.
func TestParseDurationNegativeWithSuffix(t *testing.T) {
	_, err := parseDuration("-1m")
	if err == nil {
		t.Error("parseDuration(\"-1m\") should return error")
	}
}

// TestParseDurationOnlySuffix verifies that a bare suffix returns an error.
func TestParseDurationOnlySuffix(t *testing.T) {
	suffixes := []string{"s", "m", "h", "d"}
	for _, s := range suffixes {
		_, err := parseDuration(s)
		if err == nil {
			t.Errorf("parseDuration(%q) should return error (bare suffix)", s)
		}
	}
}

// TestParseDurationWhitespace verifies that whitespace is trimmed.
func TestParseDurationWhitespace(t *testing.T) {
	val, err := parseDuration("  5  ")
	if err != nil {
		t.Fatalf("parseDuration(\"  5  \") failed: %v", err)
	}
	if val != 5.0 {
		t.Errorf("parseDuration(\"  5  \") = %f, want 5.0", val)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestSleepHelpFlag verifies that --help prints help text and returns 0.
func TestSleepHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSleep(toolSpecPath(t, "sleep"), []string{"sleep", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSleep(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runSleep(--help) produced no stdout output")
	}
}

// TestSleepVersionFlag verifies that --version prints the version.
func TestSleepVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSleep(toolSpecPath(t, "sleep"), []string{"sleep", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSleep(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runSleep(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestSleepInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestSleepInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSleep("/nonexistent/sleep.json", []string{"sleep", "1"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runSleep(bad spec) returned exit code %d, want 1", code)
	}
}

// TestRunSleepZeroDuration verifies that sleep 0 completes immediately.
func TestRunSleepZeroDuration(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSleep(toolSpecPath(t, "sleep"), []string{"sleep", "0"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSleep(0) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
}

// TestRunSleepVeryShort verifies that a very short sleep completes quickly.
func TestRunSleepVeryShort(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSleep(toolSpecPath(t, "sleep"), []string{"sleep", "0.001s"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSleep(0.001s) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
}

// TestRunSleepInvalidDuration verifies that invalid duration returns exit code 1.
func TestRunSleepInvalidDuration(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSleep(toolSpecPath(t, "sleep"), []string{"sleep", "abc"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runSleep(abc) returned exit code %d, want 1", code)
	}
}
