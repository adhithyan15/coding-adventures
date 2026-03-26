// =========================================================================
// fold — Tests
// =========================================================================
//
// These tests verify the fold tool's behavior, covering:
//
//   1. Spec loading
//   2. Default wrapping at 80 columns
//   3. Custom width (-w)
//   4. Break at spaces (-s)
//   5. Short lines (no wrapping needed)
//   6. Help and version flags
//   7. Unit tests for foldLine

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestFoldSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "fold"), []string{"fold"})
	if err != nil {
		t.Fatalf("failed to load fold.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestFoldCustomWidth(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("abcdefghij\n")
	code := runFoldWithStdin(toolSpecPath(t, "fold"), []string{"fold", "-w", "5"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runFold(-w 5) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	// "abcdefghij" should be split into "abcde\nfghij".
	output := stdout.String()
	if !strings.Contains(output, "abcde") {
		t.Errorf("fold should break at 5 chars, got: %q", output)
	}
}

func TestFoldBreakAtSpaces(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello world foo\n")
	code := runFoldWithStdin(toolSpecPath(t, "fold"), []string{"fold", "-s", "-w", "10"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runFold(-s -w 10) returned exit code %d, want 0", code)
	}
	output := stdout.String()
	// Should break at a space, not in the middle of "world".
	if strings.Contains(output, "worl\nd") {
		t.Errorf("fold -s should not break mid-word, got: %q", output)
	}
}

func TestFoldShortLine(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("short\n")
	code := runFoldWithStdin(toolSpecPath(t, "fold"), []string{"fold", "-w", "80"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runFold(short) returned exit code %d, want 0", code)
	}
	if strings.TrimSpace(stdout.String()) != "short" {
		t.Errorf("fold(short line) = %q, want %q", stdout.String(), "short\n")
	}
}

func TestFoldHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFoldWithStdin(toolSpecPath(t, "fold"), []string{"fold", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runFold(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runFold(--help) produced no stdout output")
	}
}

func TestFoldVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFoldWithStdin(toolSpecPath(t, "fold"), []string{"fold", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runFold(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runFold(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestFoldInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runFoldWithStdin("/nonexistent/fold.json", []string{"fold"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runFold(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Unit tests
// =========================================================================

func TestFoldLineExact(t *testing.T) {
	result := foldLine("abcdefghij", 5, false, false)
	if result != "abcde\nfghij" {
		t.Errorf("foldLine(10 chars, width 5) = %q, want %q", result, "abcde\nfghij")
	}
}

func TestFoldLineShort(t *testing.T) {
	result := foldLine("abc", 10, false, false)
	if result != "abc" {
		t.Errorf("foldLine(3 chars, width 10) = %q, want %q", result, "abc")
	}
}

func TestFoldLineEmpty(t *testing.T) {
	result := foldLine("", 10, false, false)
	if result != "" {
		t.Errorf("foldLine(empty) = %q, want %q", result, "")
	}
}

func TestFoldLineSpaceBreak(t *testing.T) {
	result := foldLine("hello world foo", 10, true, false)
	// Should break at the space before "world" or after it.
	if !strings.Contains(result, "\n") {
		t.Errorf("foldLine with spaces should break, got: %q", result)
	}
}
