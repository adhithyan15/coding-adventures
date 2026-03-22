// =========================================================================
// echo — Tests
// =========================================================================
//
// These tests verify the echo tool's behavior across all its modes:
//
//   1. Default: print arguments separated by spaces, followed by newline
//   2. -n: suppress trailing newline
//   3. -e: interpret backslash escape sequences
//   4. -E: disable escape interpretation (default, explicit)
//   5. --help and --version: standard meta-flags
//
// We also test the processEscapes() helper function directly, since it
// contains the most complex logic in the echo implementation.

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

// TestEchoSpecLoads verifies that echo.json is a valid cli-builder spec.
func TestEchoSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "echo"), []string{"echo"})
	if err != nil {
		t.Fatalf("failed to load echo.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Default behavior tests
// =========================================================================

// TestEchoNoArgs verifies that `echo` with no arguments prints just a
// newline — an empty line.
func TestEchoNoArgs(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	// echo with no args outputs just a newline.
	if stdout.String() != "\n" {
		t.Errorf("runEcho() output = %q, want %q", stdout.String(), "\n")
	}
}

// TestEchoSingleArg verifies that `echo hello` prints "hello\n".
func TestEchoSingleArg(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "hello"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(hello) returned exit code %d, want 0", code)
	}

	if stdout.String() != "hello\n" {
		t.Errorf("runEcho(hello) output = %q, want %q", stdout.String(), "hello\n")
	}
}

// TestEchoMultipleArgs verifies that `echo hello world` prints "hello world\n".
func TestEchoMultipleArgs(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "hello", "world"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(hello world) returned exit code %d, want 0", code)
	}

	if stdout.String() != "hello world\n" {
		t.Errorf("runEcho(hello world) output = %q, want %q", stdout.String(), "hello world\n")
	}
}

// =========================================================================
// Flag tests
// =========================================================================

// TestEchoNoNewline verifies that -n suppresses the trailing newline.
func TestEchoNoNewline(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "-n", "hello"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(-n hello) returned exit code %d, want 0", code)
	}

	// With -n, there should be no trailing newline.
	if stdout.String() != "hello" {
		t.Errorf("runEcho(-n hello) output = %q, want %q", stdout.String(), "hello")
	}
}

// TestEchoEnableEscapes verifies that -e interprets backslash escapes.
func TestEchoEnableEscapes(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "-e", `hello\nworld`}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(-e) returned exit code %d, want 0", code)
	}

	// With -e, \n should become an actual newline.
	expected := "hello\nworld\n"
	if stdout.String() != expected {
		t.Errorf("runEcho(-e hello\\nworld) output = %q, want %q", stdout.String(), expected)
	}
}

// TestEchoDisableEscapes verifies that -E keeps backslashes literal.
func TestEchoDisableEscapes(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "-E", `hello\nworld`}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(-E) returned exit code %d, want 0", code)
	}

	// With -E (default), \n should remain literal.
	expected := "hello\\nworld\n"
	if stdout.String() != expected {
		t.Errorf("runEcho(-E hello\\nworld) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestEchoHelpFlag verifies that --help prints help text and returns 0.
func TestEchoHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runEcho(--help) produced no stdout output")
	}
}

// TestEchoVersionFlag verifies that --version prints the version.
func TestEchoVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho(toolSpecPath(t, "echo"), []string{"echo", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runEcho(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runEcho(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestEchoInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestEchoInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runEcho("/nonexistent/echo.json", []string{"echo"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runEcho(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// processEscapes() unit tests
// =========================================================================
//
// These test the escape processing function directly, covering all
// supported escape sequences.

// TestProcessEscapesNewline verifies \n becomes a real newline.
func TestProcessEscapesNewline(t *testing.T) {
	result := processEscapes(`hello\nworld`)
	if result != "hello\nworld" {
		t.Errorf("processEscapes(hello\\nworld) = %q, want %q", result, "hello\nworld")
	}
}

// TestProcessEscapesTab verifies \t becomes a real tab.
func TestProcessEscapesTab(t *testing.T) {
	result := processEscapes(`hello\tworld`)
	if result != "hello\tworld" {
		t.Errorf("processEscapes(hello\\tworld) = %q, want %q", result, "hello\tworld")
	}
}

// TestProcessEscapesBackslash verifies \\ becomes a single backslash.
func TestProcessEscapesBackslash(t *testing.T) {
	result := processEscapes(`hello\\world`)
	if result != "hello\\world" {
		t.Errorf("processEscapes(hello\\\\world) = %q, want %q", result, "hello\\world")
	}
}

// TestProcessEscapesAlert verifies \a becomes the bell character.
func TestProcessEscapesAlert(t *testing.T) {
	result := processEscapes(`\a`)
	if result != "\a" {
		t.Errorf("processEscapes(\\a) = %q, want %q", result, "\a")
	}
}

// TestProcessEscapesCarriageReturn verifies \r becomes carriage return.
func TestProcessEscapesCarriageReturn(t *testing.T) {
	result := processEscapes(`\r`)
	if result != "\r" {
		t.Errorf("processEscapes(\\r) = %q, want %q", result, "\r")
	}
}

// TestProcessEscapesOctal verifies \0nnn interprets octal values.
func TestProcessEscapesOctal(t *testing.T) {
	// \0101 = 65 in decimal = 'A' in ASCII
	result := processEscapes(`\0101`)
	if result != "A" {
		t.Errorf("processEscapes(\\0101) = %q, want %q", result, "A")
	}
}

// TestProcessEscapesNoEscape verifies plain text passes through unchanged.
func TestProcessEscapesNoEscape(t *testing.T) {
	result := processEscapes("hello world")
	if result != "hello world" {
		t.Errorf("processEscapes(hello world) = %q, want %q", result, "hello world")
	}
}

// TestProcessEscapesMultiple verifies multiple escapes in one string.
func TestProcessEscapesMultiple(t *testing.T) {
	result := processEscapes(`a\tb\nc`)
	expected := "a\tb\nc"
	if result != expected {
		t.Errorf("processEscapes(a\\tb\\nc) = %q, want %q", result, expected)
	}
}

// TestProcessEscapesTrailingBackslash verifies a trailing backslash is kept.
func TestProcessEscapesTrailingBackslash(t *testing.T) {
	result := processEscapes(`hello\`)
	if result != "hello\\" {
		t.Errorf("processEscapes(hello\\) = %q, want %q", result, "hello\\")
	}
}
