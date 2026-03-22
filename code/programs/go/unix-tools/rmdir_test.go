// =========================================================================
// rmdir — Tests
// =========================================================================
//
// These tests verify the rmdir tool's behavior, covering:
//
//   1. Spec loading
//   2. Removing an empty directory
//   3. Failing to remove a non-empty directory
//   4. Removing parent directories with -p
//   5. Verbose output (-v)
//   6. Help and version flags

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestRmdirSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "rmdir"), []string{"rmdir", "test"})
	if err != nil {
		t.Fatalf("failed to load rmdir.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestRmdirRemoveEmpty(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "emptydir")
	os.Mkdir(target, 0755)

	var stdout, stderr bytes.Buffer
	code := runRmdir(toolSpecPath(t, "rmdir"), []string{"rmdir", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRmdir() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Error("directory should have been removed")
	}
}

func TestRmdirNonEmptyFails(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "nonempty")
	os.Mkdir(target, 0755)
	os.WriteFile(filepath.Join(target, "file.txt"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRmdir(toolSpecPath(t, "rmdir"), []string{"rmdir", target}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRmdir(non-empty) returned exit code %d, want 1", code)
	}
}

func TestRmdirParents(t *testing.T) {
	dir := t.TempDir()
	nested := filepath.Join(dir, "a", "b", "c")
	os.MkdirAll(nested, 0755)

	// Only remove c, then b, then a — not the temp dir itself.
	// We pass the nested path and expect it to remove up the chain
	// but stop when it hits a non-empty or root directory.
	var stdout, stderr bytes.Buffer
	code := runRmdir(toolSpecPath(t, "rmdir"), []string{"rmdir", "-p", nested}, &stdout, &stderr)

	// The -p flag will try to remove parent dirs and may fail on the
	// temp dir (which is managed by the test framework). That's OK —
	// we just verify the nested dirs were removed.
	_ = code

	if _, err := os.Stat(filepath.Join(dir, "a")); !os.IsNotExist(err) {
		t.Error("parent directory 'a' should have been removed")
	}
}

func TestRmdirVerbose(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "verbosedir")
	os.Mkdir(target, 0755)

	var stdout, stderr bytes.Buffer
	code := runRmdir(toolSpecPath(t, "rmdir"), []string{"rmdir", "-v", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRmdir(-v) returned exit code %d, want 0", code)
	}
	if !strings.Contains(stdout.String(), "removing directory") {
		t.Errorf("verbose output should contain 'removing directory', got: %q", stdout.String())
	}
}

func TestRmdirHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRmdir(toolSpecPath(t, "rmdir"), []string{"rmdir", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRmdir(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runRmdir(--help) produced no stdout output")
	}
}

func TestRmdirVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRmdir(toolSpecPath(t, "rmdir"), []string{"rmdir", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRmdir(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runRmdir(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestRmdirInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRmdir("/nonexistent/rmdir.json", []string{"rmdir", "test"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRmdir(bad spec) returned exit code %d, want 1", code)
	}
}
