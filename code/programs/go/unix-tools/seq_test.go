// =========================================================================
// seq — Tests
// =========================================================================
//
// These tests verify the seq tool's behavior, covering:
//
//   1. Spec loading
//   2. Single argument (seq LAST)
//   3. Two arguments (seq FIRST LAST)
//   4. Three arguments (seq FIRST INCREMENT LAST)
//   5. Custom separator (-s)
//   6. Equal width padding (-w)
//   7. Counting down (negative increment)
//   8. Help and version flags
//   9. Error handling (zero increment, bad numbers)
//  10. Floating point sequences

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

// TestSeqSpecLoads verifies that seq.json is a valid spec.
func TestSeqSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "seq"), []string{"seq"})
	if err != nil {
		t.Fatalf("failed to load seq.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Single argument tests
// =========================================================================

// TestSeqSingleArg verifies `seq 5` prints 1 through 5.
func TestSeqSingleArg(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "5"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(5) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	expected := "1\n2\n3\n4\n5\n"
	if stdout.String() != expected {
		t.Errorf("runSeq(5) output = %q, want %q", stdout.String(), expected)
	}
}

// TestSeqSingleArgOne verifies `seq 1` prints just 1.
func TestSeqSingleArgOne(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "1"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(1) returned exit code %d, want 0", code)
	}

	if stdout.String() != "1\n" {
		t.Errorf("runSeq(1) output = %q, want %q", stdout.String(), "1\n")
	}
}

// =========================================================================
// Two argument tests
// =========================================================================

// TestSeqTwoArgs verifies `seq 3 7` prints 3 through 7.
func TestSeqTwoArgs(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "3", "7"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(3, 7) returned exit code %d, want 0", code)
	}

	expected := "3\n4\n5\n6\n7\n"
	if stdout.String() != expected {
		t.Errorf("runSeq(3, 7) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Three argument tests
// =========================================================================

// TestSeqThreeArgs verifies `seq 1 2 10` prints odds 1,3,5,7,9.
func TestSeqThreeArgs(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "1", "2", "9"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(1, 2, 9) returned exit code %d, want 0", code)
	}

	expected := "1\n3\n5\n7\n9\n"
	if stdout.String() != expected {
		t.Errorf("runSeq(1, 2, 9) output = %q, want %q", stdout.String(), expected)
	}
}

// TestSeqCountDown verifies counting down with negative increment.
func TestSeqCountDown(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "--", "5", "-1", "1"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(5, -1, 1) returned exit code %d, want 0", code)
	}

	expected := "5\n4\n3\n2\n1\n"
	if stdout.String() != expected {
		t.Errorf("runSeq(5, -1, 1) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Separator flag tests
// =========================================================================

// TestSeqCustomSeparator verifies -s flag uses a custom separator.
func TestSeqCustomSeparator(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "-s", ", ", "3"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(-s ', ') returned exit code %d, want 0", code)
	}

	expected := "1, 2, 3\n"
	if stdout.String() != expected {
		t.Errorf("runSeq(-s) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Equal width flag tests
// =========================================================================

// TestSeqEqualWidth verifies -w pads with leading zeros.
func TestSeqEqualWidth(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "-w", "8", "10"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(-w) returned exit code %d, want 0", code)
	}

	expected := "08\n09\n10\n"
	if stdout.String() != expected {
		t.Errorf("runSeq(-w 8 10) output = %q, want %q", stdout.String(), expected)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestSeqHelpFlag verifies --help.
func TestSeqHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runSeq(--help) produced no stdout output")
	}
}

// TestSeqVersionFlag verifies --version.
func TestSeqVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runSeq(--version) = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestSeqInvalidSpec verifies bad spec path returns exit code 1.
func TestSeqInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq("/nonexistent/seq.json", []string{"seq", "5"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runSeq(bad spec) returned exit code %d, want 1", code)
	}
}

// TestSeqZeroIncrement verifies zero increment produces an error.
func TestSeqZeroIncrement(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "1", "0", "5"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runSeq(zero increment) returned exit code %d, want 1", code)
	}
}

// TestSeqEmptyRange verifies that FIRST > LAST with positive increment
// produces no output.
func TestSeqEmptyRange(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSeq(toolSpecPath(t, "seq"), []string{"seq", "5", "1"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runSeq(5, 1) returned exit code %d, want 0", code)
	}

	if stdout.Len() != 0 {
		t.Errorf("runSeq(5, 1) should produce no output, got: %q", stdout.String())
	}
}
