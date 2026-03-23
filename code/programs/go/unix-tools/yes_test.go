// =========================================================================
// yes — Tests
// =========================================================================
//
// These tests verify the yes tool's behavior:
//
//   1. Default output: prints "y" repeatedly
//   2. Custom string: prints the given string repeatedly
//   3. Multiple arguments: joins with spaces
//   4. Line limiting: yesOutput respects maxLines
//   5. --help and --version: standard meta-flags
//
// Since yes runs forever in production, we test the yesOutput function
// directly with a maxLines cap to avoid infinite loops in tests.

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

// TestYesSpecLoads verifies that yes.json is a valid cli-builder spec.
func TestYesSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "yes"), []string{"yes"})
	if err != nil {
		t.Fatalf("failed to load yes.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// yesOutput unit tests
// =========================================================================

// TestYesOutputDefault verifies that yesOutput with no args prints "y".
func TestYesOutputDefault(t *testing.T) {
	var buf bytes.Buffer
	yesOutput(nil, &buf, 3)

	expected := "y\ny\ny\n"
	if buf.String() != expected {
		t.Errorf("yesOutput(nil, 3) = %q, want %q", buf.String(), expected)
	}
}

// TestYesOutputEmptySlice verifies that an empty slice also defaults to "y".
func TestYesOutputEmptySlice(t *testing.T) {
	var buf bytes.Buffer
	yesOutput([]string{}, &buf, 2)

	expected := "y\ny\n"
	if buf.String() != expected {
		t.Errorf("yesOutput([], 2) = %q, want %q", buf.String(), expected)
	}
}

// TestYesOutputCustomString verifies that a custom string is repeated.
func TestYesOutputCustomString(t *testing.T) {
	var buf bytes.Buffer
	yesOutput([]string{"hello"}, &buf, 4)

	expected := "hello\nhello\nhello\nhello\n"
	if buf.String() != expected {
		t.Errorf("yesOutput([hello], 4) = %q, want %q", buf.String(), expected)
	}
}

// TestYesOutputMultipleArgs verifies that multiple args are joined by spaces.
func TestYesOutputMultipleArgs(t *testing.T) {
	var buf bytes.Buffer
	yesOutput([]string{"hello", "world"}, &buf, 2)

	expected := "hello world\nhello world\n"
	if buf.String() != expected {
		t.Errorf("yesOutput([hello world], 2) = %q, want %q", buf.String(), expected)
	}
}

// TestYesOutputSingleLine verifies maxLines=1 produces exactly one line.
func TestYesOutputSingleLine(t *testing.T) {
	var buf bytes.Buffer
	yesOutput([]string{"test"}, &buf, 1)

	expected := "test\n"
	if buf.String() != expected {
		t.Errorf("yesOutput([test], 1) = %q, want %q", buf.String(), expected)
	}
}

// TestYesOutputZeroLines verifies that maxLines=0 would run forever.
// We don't actually test infinite output, but we verify that a large
// number of lines are produced by checking a limited buffer.
func TestYesOutputManyLines(t *testing.T) {
	var buf bytes.Buffer
	yesOutput([]string{"x"}, &buf, 1000)

	lines := strings.Split(strings.TrimSuffix(buf.String(), "\n"), "\n")
	if len(lines) != 1000 {
		t.Errorf("yesOutput([x], 1000) produced %d lines, want 1000", len(lines))
	}

	// Verify every line is "x".
	for i, line := range lines {
		if line != "x" {
			t.Errorf("line %d = %q, want %q", i, line, "x")
			break
		}
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestYesHelpFlag verifies that --help prints help text and returns 0.
func TestYesHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runYes(toolSpecPath(t, "yes"), []string{"yes", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runYes(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runYes(--help) produced no stdout output")
	}
}

// TestYesVersionFlag verifies that --version prints the version.
func TestYesVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runYes(toolSpecPath(t, "yes"), []string{"yes", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runYes(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runYes(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestYesInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestYesInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runYes("/nonexistent/yes.json", []string{"yes"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runYes(bad spec) returned exit code %d, want 1", code)
	}
}
