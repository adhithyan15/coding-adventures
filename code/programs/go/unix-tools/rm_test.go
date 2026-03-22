// =========================================================================
// rm — Tests
// =========================================================================
//
// These tests verify the rm tool's behavior, covering:
//
//   1. Spec loading
//   2. Removing a single file
//   3. Removing multiple files
//   4. Force mode (-f) for nonexistent files
//   5. Recursive directory removal (-r)
//   6. Refusing to remove directories without -r
//   7. Verbose output (-v)
//   8. Help and version flags

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestRmSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "rm"), []string{"rm", "test"})
	if err != nil {
		t.Fatalf("failed to load rm.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestRmSingleFile(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "file.txt")
	os.WriteFile(target, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRm() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Error("file should have been removed")
	}
}

func TestRmNonexistentWithoutForce(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", "/nonexistent/file.txt"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRm(nonexistent) returned exit code %d, want 1", code)
	}
}

func TestRmNonexistentWithForce(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", "-f", "/nonexistent/file.txt"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRm(-f nonexistent) returned exit code %d, want 0", code)
	}
}

func TestRmRecursive(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "mydir")
	os.MkdirAll(filepath.Join(target, "sub"), 0755)
	os.WriteFile(filepath.Join(target, "sub", "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", "-r", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRm(-r) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}
	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Error("directory should have been removed recursively")
	}
}

func TestRmDirectoryWithoutRecursive(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "mydir")
	os.Mkdir(target, 0755)
	os.WriteFile(filepath.Join(target, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", target}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRm(dir without -r) returned exit code %d, want 1", code)
	}
}

func TestRmVerbose(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "file.txt")
	os.WriteFile(target, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", "-v", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRm(-v) returned exit code %d, want 0", code)
	}
	if !strings.Contains(stdout.String(), "removed") {
		t.Errorf("verbose output should contain 'removed', got: %q", stdout.String())
	}
}

func TestRmHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRm(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runRm(--help) produced no stdout output")
	}
}

func TestRmVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRm(toolSpecPath(t, "rm"), []string{"rm", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRm(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runRm(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestRmInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRm("/nonexistent/rm.json", []string{"rm", "test"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRm(bad spec) returned exit code %d, want 1", code)
	}
}
