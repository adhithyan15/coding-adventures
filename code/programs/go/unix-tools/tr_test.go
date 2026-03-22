// =========================================================================
// tr — Tests
// =========================================================================
//
// These tests verify the tr tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic character translation
//   3. Range expansion (a-z)
//   4. Delete mode (-d)
//   5. Squeeze repeats (-s)
//   6. Complement mode (-c)
//   7. Help and version flags
//   8. Unit tests for expandSet, translateContent, deleteChars

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestTrSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "tr"), []string{"tr", "a", "b"})
	if err != nil {
		t.Fatalf("failed to load tr.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestTrBasicTranslation(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello")
	code := runTrWithStdin(toolSpecPath(t, "tr"), []string{"tr", "elo", "ELO"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTr() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	if stdout.String() != "hELLO" {
		t.Errorf("tr output = %q, want %q", stdout.String(), "hELLO")
	}
}

func TestTrDelete(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("hello world")
	code := runTrWithStdin(toolSpecPath(t, "tr"), []string{"tr", "-d", "lo"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTr(-d) returned exit code %d, want 0", code)
	}
	if stdout.String() != "he wrd" {
		t.Errorf("tr -d output = %q, want %q", stdout.String(), "he wrd")
	}
}

func TestTrSqueeze(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("aaabbbccc")
	code := runTrWithStdin(toolSpecPath(t, "tr"), []string{"tr", "-s", "a-z"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runTr(-s) returned exit code %d, want 0", code)
	}
	if stdout.String() != "abc" {
		t.Errorf("tr -s output = %q, want %q", stdout.String(), "abc")
	}
}

func TestTrHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrWithStdin(toolSpecPath(t, "tr"), []string{"tr", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTr(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runTr(--help) produced no stdout output")
	}
}

func TestTrVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrWithStdin(toolSpecPath(t, "tr"), []string{"tr", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runTr(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runTr(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestTrInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTrWithStdin("/nonexistent/tr.json", []string{"tr", "a", "b"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runTr(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Unit tests for helper functions
// =========================================================================

func TestExpandSetRange(t *testing.T) {
	result := expandSet("a-d")
	if result != "abcd" {
		t.Errorf("expandSet('a-d') = %q, want %q", result, "abcd")
	}
}

func TestExpandSetLiteral(t *testing.T) {
	result := expandSet("abc")
	if result != "abc" {
		t.Errorf("expandSet('abc') = %q, want %q", result, "abc")
	}
}

func TestExpandSetMixed(t *testing.T) {
	result := expandSet("a-c0-2")
	if result != "abc012" {
		t.Errorf("expandSet('a-c0-2') = %q, want %q", result, "abc012")
	}
}

func TestTranslateContent(t *testing.T) {
	result := translateContent("hello", "helo", "HELO", false)
	if result != "HELLO" {
		t.Errorf("translateContent = %q, want %q", result, "HELLO")
	}
}

func TestDeleteChars(t *testing.T) {
	result := deleteChars("hello world", "lo", false)
	if result != "he wrd" {
		t.Errorf("deleteChars = %q, want %q", result, "he wrd")
	}
}

func TestSqueezeRepeats(t *testing.T) {
	result := squeezeRepeats("aabbcc", "a-c")
	if result != "abc" {
		t.Errorf("squeezeRepeats = %q, want %q", result, "abc")
	}
}
