// =========================================================================
// uniq — Tests
// =========================================================================
//
// These tests verify the uniq tool's behavior, covering:
//
//   1. Spec loading
//   2. Default deduplication of adjacent lines
//   3. Count mode (-c)
//   4. Repeated only mode (-d)
//   5. Unique only mode (-u)
//   6. Case-insensitive mode (-i)
//   7. Help and version flags
//   8. Unit tests for processUniq and compareKey

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestUniqSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "uniq"), []string{"uniq"})
	if err != nil {
		t.Fatalf("failed to load uniq.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestUniqDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("apple\napple\nbanana\napple\n")
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUniq() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	expected := "apple\nbanana\napple\n"
	if stdout.String() != expected {
		t.Errorf("runUniq() output = %q, want %q", stdout.String(), expected)
	}
}

func TestUniqCount(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("apple\napple\nbanana\n")
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq", "-c"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUniq(-c) returned exit code %d, want 0", code)
	}
	output := stdout.String()
	if !strings.Contains(output, "2 apple") {
		t.Errorf("count output should contain '2 apple', got: %q", output)
	}
	if !strings.Contains(output, "1 banana") {
		t.Errorf("count output should contain '1 banana', got: %q", output)
	}
}

func TestUniqRepeatedOnly(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("apple\napple\nbanana\n")
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq", "-d"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUniq(-d) returned exit code %d, want 0", code)
	}
	expected := "apple\n"
	if stdout.String() != expected {
		t.Errorf("runUniq(-d) output = %q, want %q", stdout.String(), expected)
	}
}

func TestUniqUniqueOnly(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("apple\napple\nbanana\n")
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq", "-u"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUniq(-u) returned exit code %d, want 0", code)
	}
	expected := "banana\n"
	if stdout.String() != expected {
		t.Errorf("runUniq(-u) output = %q, want %q", stdout.String(), expected)
	}
}

func TestUniqIgnoreCase(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("Apple\napple\nBANANA\n")
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq", "-i"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runUniq(-i) returned exit code %d, want 0", code)
	}
	expected := "Apple\nBANANA\n"
	if stdout.String() != expected {
		t.Errorf("runUniq(-i) output = %q, want %q", stdout.String(), expected)
	}
}

func TestUniqHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runUniq(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runUniq(--help) produced no stdout output")
	}
}

func TestUniqVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUniqWithStdin(toolSpecPath(t, "uniq"), []string{"uniq", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runUniq(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runUniq(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestUniqInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUniqWithStdin("/nonexistent/uniq.json", []string{"uniq"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runUniq(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Unit tests for helper functions
// =========================================================================

func TestProcessUniqBasic(t *testing.T) {
	result := processUniq("a\na\nb\n", false, false, false, false, 0, 0, 0)
	if result != "a\nb\n" {
		t.Errorf("processUniq basic = %q, want %q", result, "a\nb\n")
	}
}

func TestProcessUniqEmpty(t *testing.T) {
	result := processUniq("", false, false, false, false, 0, 0, 0)
	if result != "" {
		t.Errorf("processUniq empty = %q, want %q", result, "")
	}
}

func TestCompareKeySkipFields(t *testing.T) {
	key := compareKey("field1 field2 data", 2, 0, 0, false)
	if key != "data" {
		t.Errorf("compareKey(skip 2 fields) = %q, want %q", key, "data")
	}
}

func TestCompareKeyIgnoreCase(t *testing.T) {
	key := compareKey("HELLO", 0, 0, 0, true)
	if key != "hello" {
		t.Errorf("compareKey(ignoreCase) = %q, want %q", key, "hello")
	}
}
