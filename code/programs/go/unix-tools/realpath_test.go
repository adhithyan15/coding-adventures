// =========================================================================
// realpath — Tests
// =========================================================================
//
// These tests verify the realpath tool's behavior, covering:
//
//   1. Spec loading
//   2. Resolving an absolute path
//   3. Resolving a relative path
//   4. Symlink resolution
//   5. -e flag (all must exist)
//   6. -m flag (nothing need exist)
//   7. Help and version flags

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestRealpathSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "realpath"), []string{"realpath", "."})
	if err != nil {
		t.Fatalf("failed to load realpath.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestRealpathExistingFile(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "file.txt")
	os.WriteFile(target, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	code := runRealpath(toolSpecPath(t, "realpath"), []string{"realpath", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRealpath() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if !filepath.IsAbs(output) {
		t.Errorf("output should be absolute path, got: %q", output)
	}
}

func TestRealpathSymlink(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "real.txt")
	link := filepath.Join(dir, "link.txt")
	os.WriteFile(target, []byte("data"), 0644)
	os.Symlink(target, link)

	var stdout, stderr bytes.Buffer
	code := runRealpath(toolSpecPath(t, "realpath"), []string{"realpath", link}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRealpath(symlink) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	// The resolved path should match the target (not the link).
	resolvedTarget, _ := filepath.EvalSymlinks(target)
	resolvedTarget, _ = filepath.Abs(resolvedTarget)
	if output != resolvedTarget {
		t.Errorf("realpath(symlink) = %q, want %q", output, resolvedTarget)
	}
}

func TestRealpathCanonicalizeExisting(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRealpath(toolSpecPath(t, "realpath"), []string{"realpath", "-e", "/nonexistent/path"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRealpath(-e nonexistent) returned exit code %d, want 1", code)
	}
}

func TestRealpathCanonicalizeMissing(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRealpath(toolSpecPath(t, "realpath"), []string{"realpath", "-m", "/nonexistent/path/to/file"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRealpath(-m) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output != "/nonexistent/path/to/file" {
		t.Errorf("realpath(-m) = %q, want %q", output, "/nonexistent/path/to/file")
	}
}

func TestRealpathHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRealpath(toolSpecPath(t, "realpath"), []string{"realpath", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRealpath(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runRealpath(--help) produced no stdout output")
	}
}

func TestRealpathVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRealpath(toolSpecPath(t, "realpath"), []string{"realpath", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runRealpath(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runRealpath(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestRealpathInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runRealpath("/nonexistent/realpath.json", []string{"realpath", "."}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runRealpath(bad spec) returned exit code %d, want 1", code)
	}
}
