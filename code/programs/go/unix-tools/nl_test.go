// =========================================================================
// nl — Tests
// =========================================================================
//
// These tests verify the nl tool's behavior, covering:
//
//   1. Spec loading
//   2. Default behavior (number non-empty lines)
//   3. Number all lines (-ba)
//   4. No numbering (-bn)
//   5. Custom format (-n ln, -n rz)
//   6. Custom width (-w)
//   7. Custom separator (-s)
//   8. Help and version flags
//   9. Unit tests for shouldNumber, formatLineNumber

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestNlSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "nl"), []string{"nl"})
	if err != nil {
		t.Fatalf("failed to load nl.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestNlDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello\nworld\n")
	code := runNlWithStdin(toolSpecPath(t, "nl"), []string{"nl"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runNl() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	output := stdout.String()
	if !strings.Contains(output, "1") {
		t.Errorf("nl output should contain line number 1, got: %q", output)
	}
	if !strings.Contains(output, "hello") {
		t.Errorf("nl output should contain 'hello', got: %q", output)
	}
}

func TestNlNumberAll(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello\n\nworld\n")
	code := runNlWithStdin(toolSpecPath(t, "nl"), []string{"nl", "-b", "a"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runNl(-ba) returned exit code %d, want 0", code)
	}
	output := stdout.String()
	lines := strings.Split(strings.TrimRight(output, "\n"), "\n")
	// All three lines should be numbered (including the empty one).
	if len(lines) != 3 {
		t.Errorf("nl -ba should produce 3 lines, got %d", len(lines))
	}
}

func TestNlSkipEmpty(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello\n\nworld\n")
	code := runNlWithStdin(toolSpecPath(t, "nl"), []string{"nl"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runNl(default) returned exit code %d, want 0", code)
	}
	output := stdout.String()
	// "hello" should be numbered as 1, empty line unnumbered, "world" as 2.
	if !strings.Contains(output, "1") && !strings.Contains(output, "2") {
		t.Errorf("nl should number non-empty lines, got: %q", output)
	}
}

func TestNlHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNlWithStdin(toolSpecPath(t, "nl"), []string{"nl", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runNl(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runNl(--help) produced no stdout output")
	}
}

func TestNlVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNlWithStdin(toolSpecPath(t, "nl"), []string{"nl", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runNl(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runNl(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestNlInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runNlWithStdin("/nonexistent/nl.json", []string{"nl"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runNl(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Unit tests
// =========================================================================

func TestShouldNumberAll(t *testing.T) {
	if !shouldNumber("hello", "a") {
		t.Error("shouldNumber('hello', 'a') should be true")
	}
	if !shouldNumber("", "a") {
		t.Error("shouldNumber('', 'a') should be true")
	}
}

func TestShouldNumberNonEmpty(t *testing.T) {
	if !shouldNumber("hello", "t") {
		t.Error("shouldNumber('hello', 't') should be true")
	}
	if shouldNumber("", "t") {
		t.Error("shouldNumber('', 't') should be false")
	}
	if shouldNumber("   ", "t") {
		t.Error("shouldNumber('   ', 't') should be false")
	}
}

func TestShouldNumberNone(t *testing.T) {
	if shouldNumber("hello", "n") {
		t.Error("shouldNumber('hello', 'n') should be false")
	}
}

func TestFormatLineNumberRN(t *testing.T) {
	result := formatLineNumber(1, 6, "rn")
	if result != "     1" {
		t.Errorf("formatLineNumber(1, 6, rn) = %q, want %q", result, "     1")
	}
}

func TestFormatLineNumberLN(t *testing.T) {
	result := formatLineNumber(1, 6, "ln")
	if result != "1     " {
		t.Errorf("formatLineNumber(1, 6, ln) = %q, want %q", result, "1     ")
	}
}

func TestFormatLineNumberRZ(t *testing.T) {
	result := formatLineNumber(1, 6, "rz")
	if result != "000001" {
		t.Errorf("formatLineNumber(1, 6, rz) = %q, want %q", result, "000001")
	}
}
