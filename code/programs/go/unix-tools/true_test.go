// =========================================================================
// true — Tests
// =========================================================================
//
// These tests verify the `true` tool's behavior. Despite being one of the
// simplest Unix utilities, we test it thoroughly to ensure:
//
//   1. The spec file loads correctly.
//   2. Normal execution returns exit code 0.
//   3. --help prints help text and returns 0.
//   4. --version prints "1.0.0" and returns 0.
//   5. An invalid spec path returns exit code 1.
//
// Since `true` has no flags or arguments (beyond --help/--version),
// the tests are straightforward.

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

// TestTrueSpecLoads verifies that true.json is a valid cli-builder spec.
func TestTrueSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "true"), []string{"true"})
	if err != nil {
		t.Fatalf("failed to load true.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestTrueDefaultExitCode verifies that running `true` with no args
// returns exit code 0.
func TestTrueDefaultExitCode(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrue(toolSpecPath(t, "true"), []string{"true"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTrue() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
}

// TestTrueDefaultNoOutput verifies that `true` produces no output when
// run without flags.
func TestTrueDefaultNoOutput(t *testing.T) {
	var stdout, stderr bytes.Buffer
	runTrue(toolSpecPath(t, "true"), []string{"true"}, &stdout, &stderr)

	if stdout.Len() != 0 {
		t.Errorf("runTrue() produced unexpected stdout: %q", stdout.String())
	}
	if stderr.Len() != 0 {
		t.Errorf("runTrue() produced unexpected stderr: %q", stderr.String())
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestTrueHelpFlag verifies that --help prints help text and returns 0.
func TestTrueHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrue(toolSpecPath(t, "true"), []string{"true", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTrue(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runTrue(--help) produced no stdout output")
	}

	// The help text should mention the tool's purpose.
	if !strings.Contains(stdout.String(), "successfully") {
		t.Error("help text should mention 'successfully'")
	}
}

// TestTrueVersionFlag verifies that --version prints the version and returns 0.
func TestTrueVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrue(toolSpecPath(t, "true"), []string{"true", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTrue(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runTrue(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestTrueInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestTrueInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrue("/nonexistent/true.json", []string{"true"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runTrue(bad spec) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("runTrue(bad spec) should produce stderr output")
	}
}

// =========================================================================
// Parser integration tests
// =========================================================================

// TestTrueParseResult verifies that normal parsing produces a ParseResult.
func TestTrueParseResult(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "true"), []string{"true"})
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

	if pr.Program != "true" {
		t.Errorf("Program = %q, want %q", pr.Program, "true")
	}
}

// TestTrueHelpResult verifies that --help produces a HelpResult.
func TestTrueHelpResult(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "true"), []string{"true", "--help"})
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
