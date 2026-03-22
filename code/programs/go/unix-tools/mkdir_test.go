// =========================================================================
// mkdir — Tests
// =========================================================================
//
// These tests verify the mkdir tool's behavior, covering:
//
//   1. Spec loading
//   2. Creating a single directory
//   3. Creating nested directories with -p
//   4. Error when parent doesn't exist (without -p)
//   5. Verbose output (-v)
//   6. Help and version flags
//   7. Error handling for invalid paths

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

func TestMkdirSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "mkdir"), []string{"mkdir", "test"})
	if err != nil {
		t.Fatalf("failed to load mkdir.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Basic directory creation tests
// =========================================================================

func TestMkdirCreateSingle(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "newdir")

	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMkdir() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	info, err := os.Stat(target)
	if err != nil {
		t.Fatalf("directory was not created: %v", err)
	}
	if !info.IsDir() {
		t.Error("created path is not a directory")
	}
}

func TestMkdirParents(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "a", "b", "c")

	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", "-p", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMkdir(-p) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	info, err := os.Stat(target)
	if err != nil {
		t.Fatalf("nested directory was not created: %v", err)
	}
	if !info.IsDir() {
		t.Error("created path is not a directory")
	}
}

func TestMkdirNoParentsFails(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "nonexistent", "child")

	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", target}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runMkdir(no parents) returned exit code %d, want 1", code)
	}
}

func TestMkdirVerbose(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "verbosedir")

	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", "-v", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMkdir(-v) returned exit code %d, want 0", code)
	}

	if !strings.Contains(stdout.String(), "created directory") {
		t.Errorf("verbose output should contain 'created directory', got: %q", stdout.String())
	}
}

func TestMkdirExistingWithoutParents(t *testing.T) {
	dir := t.TempDir()

	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", dir}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runMkdir(existing) returned exit code %d, want 1", code)
	}
}

func TestMkdirExistingWithParents(t *testing.T) {
	dir := t.TempDir()

	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", "-p", dir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMkdir(-p existing) returned exit code %d, want 0", code)
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

func TestMkdirHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMkdir(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runMkdir(--help) produced no stdout output")
	}
}

func TestMkdirVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMkdir(toolSpecPath(t, "mkdir"), []string{"mkdir", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runMkdir(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runMkdir(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestMkdirInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMkdir("/nonexistent/mkdir.json", []string{"mkdir", "test"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runMkdir(bad spec) returned exit code %d, want 1", code)
	}
}
