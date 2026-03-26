package main

// main_test.go tests the run() function (the testable core of main.go).
//
// run() is separated from main() so that tests can exercise the CLI logic
// without triggering os.Exit, which would terminate the test process.

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestRun_PassingPackage verifies that run() returns exit code 0 for a package
// with no OS calls and no manifest.
func TestRun_PassingPackage(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "foo.go"), []byte(`package foo
func Add(a, b int) int { return a + b }
`), 0o600); err != nil {
		t.Fatal(err)
	}

	var stdout, stderr bytes.Buffer
	code := run(dir, false, &stdout, &stderr)
	if code != 0 {
		t.Errorf("expected exit code 0 (pass), got %d. stdout=%q stderr=%q", code, stdout.String(), stderr.String())
	}
}

// TestRun_ViolationExitCode verifies that run() returns exit code 1 when there
// are violations.
func TestRun_ViolationExitCode(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "foo.go"), []byte(`package foo
import "os"
func f() { os.ReadFile("x") }
`), 0o600); err != nil {
		t.Fatal(err)
	}
	// No manifest → undeclared capability → CAP001 violation.

	var stdout, stderr bytes.Buffer
	code := run(dir, false, &stdout, &stderr)
	if code != 1 {
		t.Errorf("expected exit code 1 (violations), got %d", code)
	}
	if !strings.Contains(stdout.String(), "CAP001") {
		t.Errorf("expected CAP001 in output, got: %q", stdout.String())
	}
}

// TestRun_ToolError verifies that run() returns exit code 2 when AnalyzeDir fails
// (e.g., nonexistent directory).
func TestRun_ToolError(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run("/nonexistent/path/12345", false, &stdout, &stderr)
	if code != 2 {
		t.Errorf("expected exit code 2 (tool error), got %d", code)
	}
	if !strings.Contains(stderr.String(), "ca-capability-analyzer") {
		t.Errorf("expected error message in stderr, got: %q", stderr.String())
	}
}

// TestRun_Verbose verifies that run() with verbose=true prints detected capabilities
// even when the package passes.
func TestRun_Verbose(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "foo.go"), []byte(`package foo
import "os"
func f() { os.ReadFile("x") }
`), 0o600); err != nil {
		t.Fatal(err)
	}
	// Add a manifest declaring fs:read:* so the package passes.
	if err := os.WriteFile(filepath.Join(dir, "required_capabilities.json"), []byte(`{
		"version": 1, "package": "go/test",
		"capabilities": [{"category": "fs", "action": "read", "target": "*", "justification": "reads files"}],
		"justification": "reads files"
	}`), 0o600); err != nil {
		t.Fatal(err)
	}

	var stdout, stderr bytes.Buffer
	code := run(dir, true, &stdout, &stderr) // verbose=true
	if code != 0 {
		t.Errorf("expected exit code 0 (pass), got %d. stdout=%q", code, stdout.String())
	}
	out := stdout.String()
	// Verbose mode should print detected capabilities and "passed".
	if !strings.Contains(out, "passed") {
		t.Errorf("expected 'passed' in verbose output, got: %q", out)
	}
}

// TestRun_ParseWarning verifies that run() writes parse error warnings to stderr
// when a file in the directory is syntactically invalid (e.g., build-tag-only
// files or files with deliberate syntax errors).
func TestRun_ParseWarning(t *testing.T) {
	dir := t.TempDir()
	// Write a valid file so AnalyzeDir has at least one parseable file.
	if err := os.WriteFile(filepath.Join(dir, "good.go"), []byte(`package foo
func Add(a, b int) int { return a + b }
`), 0o600); err != nil {
		t.Fatal(err)
	}
	// Write a syntactically invalid .go file.
	if err := os.WriteFile(filepath.Join(dir, "bad.go"), []byte(`package foo
func (((( broken syntax
`), 0o600); err != nil {
		t.Fatal(err)
	}

	var stdout, stderr bytes.Buffer
	// Even with a parse error, the overall result should still be 0 (pass) because
	// the parseable file has no violations. The parse warning goes to stderr.
	code := run(dir, false, &stdout, &stderr)
	if code != 0 {
		t.Errorf("expected exit code 0 (pass despite parse warning), got %d. stdout=%q stderr=%q",
			code, stdout.String(), stderr.String())
	}
	if !strings.Contains(stderr.String(), "warning") {
		t.Errorf("expected parse warning in stderr, got: %q", stderr.String())
	}
	if !strings.Contains(stderr.String(), "bad.go") {
		t.Errorf("expected filename in stderr warning, got: %q", stderr.String())
	}
}

// TestRun_VerboseWithBanned verifies that run() with violations prints banned constructs.
func TestRun_VerboseWithBanned(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "foo.go"), []byte(`package foo
import "C"
`), 0o600); err != nil {
		t.Fatal(err)
	}

	var stdout, stderr bytes.Buffer
	code := run(dir, true, &stdout, &stderr)
	if code != 1 {
		t.Errorf("expected exit code 1 (violations), got %d", code)
	}
	out := stdout.String()
	if !strings.Contains(out, "CGo") {
		t.Errorf("expected CGo in verbose output, got: %q", out)
	}
}
