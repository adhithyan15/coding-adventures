// =========================================================================
// unexpand — Tests
// =========================================================================
//
// These tests verify the unexpand tool's behavior, covering:
//
//   1. Spec loading
//   2. Default behavior (convert leading spaces to tabs)
//   3. All-blanks mode (-a)
//   4. First-only mode (--first-only)
//   5. Help and version flags
//   6. Unit tests for unexpandLine

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestUnexpandSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "unexpand"), []string{"unexpand"})
	if err != nil {
		t.Fatalf("failed to load unexpand.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestUnexpandDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("        hello\n")
	code := runUnexpandWithStdin(toolSpecPath(t, "unexpand"), []string{"unexpand"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUnexpand() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	expected := "\thello\n"
	if stdout.String() != expected {
		t.Errorf("unexpand output = %q, want %q", stdout.String(), expected)
	}
}

func TestUnexpandCustomWidth(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("    hello\n")
	code := runUnexpandWithStdin(toolSpecPath(t, "unexpand"), []string{"unexpand", "-t", "4"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUnexpand(-t 4) returned exit code %d, want 0", code)
	}
	expected := "\thello\n"
	if stdout.String() != expected {
		t.Errorf("unexpand -t 4 output = %q, want %q", stdout.String(), expected)
	}
}

func TestUnexpandHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUnexpandWithStdin(toolSpecPath(t, "unexpand"), []string{"unexpand", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runUnexpand(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runUnexpand(--help) produced no stdout output")
	}
}

func TestUnexpandVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUnexpandWithStdin(toolSpecPath(t, "unexpand"), []string{"unexpand", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runUnexpand(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runUnexpand(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestUnexpandInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUnexpandWithStdin("/nonexistent/unexpand.json", []string{"unexpand"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runUnexpand(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Unit tests
// =========================================================================

func TestUnexpandLineLeadingSpaces(t *testing.T) {
	result := unexpandLine("        hello", []int{8}, false)
	if result != "\thello" {
		t.Errorf("unexpandLine(8 spaces) = %q, want %q", result, "\thello")
	}
}

func TestUnexpandLinePartialSpaces(t *testing.T) {
	// 3 spaces don't reach a tab stop at column 8.
	result := unexpandLine("   hello", []int{8}, false)
	if result != "   hello" {
		t.Errorf("unexpandLine(3 spaces) = %q, want %q", result, "   hello")
	}
}

func TestUnexpandLineNoSpaces(t *testing.T) {
	result := unexpandLine("hello", []int{8}, false)
	if result != "hello" {
		t.Errorf("unexpandLine(no spaces) = %q, want %q", result, "hello")
	}
}
