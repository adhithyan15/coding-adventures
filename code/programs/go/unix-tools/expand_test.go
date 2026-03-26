// =========================================================================
// expand — Tests
// =========================================================================
//
// These tests verify the expand tool's behavior, covering:
//
//   1. Spec loading
//   2. Default tab expansion (8 columns)
//   3. Custom tab width (-t)
//   4. Initial-only mode (-i)
//   5. Help and version flags
//   6. Unit tests for expandLine, parseTabStops, nextTabStop

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestExpandSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "expand"), []string{"expand"})
	if err != nil {
		t.Fatalf("failed to load expand.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestExpandDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("\thello\n")
	code := runExpandWithStdin(toolSpecPath(t, "expand"), []string{"expand"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runExpand() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	// A tab at column 0 with 8-column stops should produce 8 spaces.
	expected := "        hello\n"
	if stdout.String() != expected {
		t.Errorf("expand output = %q, want %q", stdout.String(), expected)
	}
}

func TestExpandCustomWidth(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("\thello\n")
	code := runExpandWithStdin(toolSpecPath(t, "expand"), []string{"expand", "-t", "4"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runExpand(-t 4) returned exit code %d, want 0", code)
	}
	expected := "    hello\n"
	if stdout.String() != expected {
		t.Errorf("expand -t 4 output = %q, want %q", stdout.String(), expected)
	}
}

func TestExpandInitialOnly(t *testing.T) {
	var stdout, stderr bytes.Buffer
	stdin := strings.NewReader("\thello\tworld\n")
	code := runExpandWithStdin(toolSpecPath(t, "expand"), []string{"expand", "-i", "-t", "4"}, &stdout, &stderr, stdin)

	if code != 0 {
		t.Errorf("runExpand(-i) returned exit code %d, want 0", code)
	}
	output := stdout.String()
	// The initial tab should be expanded, but the tab after "hello" should remain.
	if !strings.HasPrefix(output, "    hello") {
		t.Errorf("expand -i should expand initial tab, got: %q", output)
	}
	if !strings.Contains(output, "\t") {
		t.Errorf("expand -i should preserve non-initial tabs, got: %q", output)
	}
}

func TestExpandHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runExpandWithStdin(toolSpecPath(t, "expand"), []string{"expand", "--help"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runExpand(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runExpand(--help) produced no stdout output")
	}
}

func TestExpandVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runExpandWithStdin(toolSpecPath(t, "expand"), []string{"expand", "--version"}, &stdout, &stderr, strings.NewReader(""))

	if code != 0 {
		t.Errorf("runExpand(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runExpand(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestExpandInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runExpandWithStdin("/nonexistent/expand.json", []string{"expand"}, &stdout, &stderr, strings.NewReader(""))

	if code != 1 {
		t.Errorf("runExpand(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Unit tests
// =========================================================================

func TestExpandLineDefault(t *testing.T) {
	result := expandLine("\thello", []int{8}, false)
	if result != "        hello" {
		t.Errorf("expandLine(tab at 0) = %q, want %q", result, "        hello")
	}
}

func TestExpandLineMidLine(t *testing.T) {
	result := expandLine("abc\tdef", []int{8}, false)
	// "abc" takes 3 columns, tab at column 3 needs 5 spaces to reach column 8.
	if result != "abc     def" {
		t.Errorf("expandLine(abc tab def) = %q, want %q", result, "abc     def")
	}
}

func TestParseTabStopsDefault(t *testing.T) {
	stops, err := parseTabStops("")
	if err != nil {
		t.Fatalf("parseTabStops('') failed: %v", err)
	}
	if len(stops) != 1 || stops[0] != 8 {
		t.Errorf("parseTabStops('') = %v, want [8]", stops)
	}
}

func TestParseTabStopsCustom(t *testing.T) {
	stops, err := parseTabStops("4")
	if err != nil {
		t.Fatalf("parseTabStops('4') failed: %v", err)
	}
	if len(stops) != 1 || stops[0] != 4 {
		t.Errorf("parseTabStops('4') = %v, want [4]", stops)
	}
}

func TestNextTabStopInterval(t *testing.T) {
	spaces := nextTabStop(0, []int{8})
	if spaces != 8 {
		t.Errorf("nextTabStop(0, [8]) = %d, want 8", spaces)
	}

	spaces = nextTabStop(3, []int{8})
	if spaces != 5 {
		t.Errorf("nextTabStop(3, [8]) = %d, want 5", spaces)
	}
}
