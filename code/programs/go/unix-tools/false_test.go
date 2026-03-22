// =========================================================================
// false — Tests
// =========================================================================
//
// These tests verify the `false` tool's behavior. The key property that
// distinguishes `false` from `true` is the exit code:
//
//   true  -> exit 0 (success)
//   false -> exit 1 (failure)
//
// Both tools respond identically to --help and --version (exit 0), and
// both produce no output during normal operation.

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

// TestFalseSpecLoads verifies that false.json is a valid cli-builder spec.
func TestFalseSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "false"), []string{"false"})
	if err != nil {
		t.Fatalf("failed to load false.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestFalseDefaultExitCode verifies that running `false` with no args
// returns exit code 1. This is the defining behavior of `false`.
func TestFalseDefaultExitCode(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFalse(toolSpecPath(t, "false"), []string{"false"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runFalse() returned exit code %d, want 1", code)
	}
}

// TestFalseDefaultNoOutput verifies that `false` produces no output
// when run without flags, just like `true`.
func TestFalseDefaultNoOutput(t *testing.T) {
	var stdout, stderr bytes.Buffer
	runFalse(toolSpecPath(t, "false"), []string{"false"}, &stdout, &stderr)

	if stdout.Len() != 0 {
		t.Errorf("runFalse() produced unexpected stdout: %q", stdout.String())
	}
	if stderr.Len() != 0 {
		t.Errorf("runFalse() produced unexpected stderr: %q", stderr.String())
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestFalseHelpFlag verifies that --help prints help text and returns 0.
// Even though `false` normally returns 1, --help is a successful operation
// (the user asked for help and got it).
func TestFalseHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFalse(toolSpecPath(t, "false"), []string{"false", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runFalse(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runFalse(--help) produced no stdout output")
	}

	// The help text should mention the tool's purpose.
	if !strings.Contains(stdout.String(), "unsuccessfully") {
		t.Error("help text should mention 'unsuccessfully'")
	}
}

// TestFalseVersionFlag verifies that --version prints the version and
// returns 0 (the version query succeeded).
func TestFalseVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFalse(toolSpecPath(t, "false"), []string{"false", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runFalse(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runFalse(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestFalseInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestFalseInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFalse("/nonexistent/false.json", []string{"false"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runFalse(bad spec) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("runFalse(bad spec) should produce stderr output")
	}
}

// =========================================================================
// Parser integration tests
// =========================================================================

// TestFalseParseResult verifies that normal parsing produces a ParseResult.
func TestFalseParseResult(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "false"), []string{"false"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	if pr.Program != "false" {
		t.Errorf("Program = %q, want %q", pr.Program, "false")
	}
}

// TestFalseHelpResult verifies that --help produces a HelpResult.
func TestFalseHelpResult(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "false"), []string{"false", "--help"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	hr, ok := result.(*clibuilder.HelpResult)
	if !ok {
		t.Fatalf("expected *HelpResult, got %T", result)
	}

	if hr.Text == "" {
		t.Error("help text should not be empty")
	}
}

// TestFalseExitCodeDiffersFromTrue is a meta-test that verifies the key
// difference between `true` and `false`: their exit codes.
func TestFalseExitCodeDiffersFromTrue(t *testing.T) {
	var stdout1, stderr1 bytes.Buffer
	trueCode := runTrue(toolSpecPath(t, "true"), []string{"true"}, &stdout1, &stderr1)

	var stdout2, stderr2 bytes.Buffer
	falseCode := runFalse(toolSpecPath(t, "false"), []string{"false"}, &stdout2, &stderr2)

	if trueCode == falseCode {
		t.Errorf("true and false should have different exit codes, both returned %d", trueCode)
	}

	if trueCode != 0 {
		t.Errorf("true exit code = %d, want 0", trueCode)
	}
	if falseCode != 1 {
		t.Errorf("false exit code = %d, want 1", falseCode)
	}
}
