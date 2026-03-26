// =========================================================================
// ln — Tests
// =========================================================================
//
// These tests verify the ln tool's behavior, covering:
//
//   1. Spec loading
//   2. Creating hard links
//   3. Creating symbolic links (-s)
//   4. Force overwrite (-f)
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

func TestLnSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "ln"), []string{"ln", "target"})
	if err != nil {
		t.Fatalf("failed to load ln.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestLnSymbolicLink(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "target.txt")
	link := filepath.Join(dir, "link.txt")
	os.WriteFile(target, []byte("hello"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLn(toolSpecPath(t, "ln"), []string{"ln", "-s", target, link}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLn(-s) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	// Verify the symlink exists and points to the target.
	linkTarget, err := os.Readlink(link)
	if err != nil {
		t.Fatalf("symlink was not created: %v", err)
	}
	if linkTarget != target {
		t.Errorf("symlink points to %q, want %q", linkTarget, target)
	}
}

func TestLnHardLink(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "target.txt")
	link := filepath.Join(dir, "hardlink.txt")
	os.WriteFile(target, []byte("hello"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLn(toolSpecPath(t, "ln"), []string{"ln", target, link}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLn(hard) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	// Verify the hard link exists and has the same content.
	content, err := os.ReadFile(link)
	if err != nil {
		t.Fatalf("hard link was not created: %v", err)
	}
	if string(content) != "hello" {
		t.Errorf("hard link content = %q, want %q", string(content), "hello")
	}
}

func TestLnForce(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "target.txt")
	link := filepath.Join(dir, "link.txt")
	os.WriteFile(target, []byte("hello"), 0644)
	os.WriteFile(link, []byte("existing"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLn(toolSpecPath(t, "ln"), []string{"ln", "-sf", target, link}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLn(-sf) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	linkTarget, err := os.Readlink(link)
	if err != nil {
		t.Fatalf("symlink was not created: %v", err)
	}
	if linkTarget != target {
		t.Errorf("symlink points to %q, want %q", linkTarget, target)
	}
}

func TestLnVerbose(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "target.txt")
	link := filepath.Join(dir, "verbose_link.txt")
	os.WriteFile(target, []byte("hello"), 0644)

	var stdout, stderr bytes.Buffer
	code := runLn(toolSpecPath(t, "ln"), []string{"ln", "-sv", target, link}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLn(-sv) returned exit code %d, want 0", code)
	}
	if !strings.Contains(stdout.String(), "->") {
		t.Errorf("verbose output should contain '->', got: %q", stdout.String())
	}
}

func TestLnHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLn(toolSpecPath(t, "ln"), []string{"ln", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLn(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runLn(--help) produced no stdout output")
	}
}

func TestLnVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLn(toolSpecPath(t, "ln"), []string{"ln", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runLn(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runLn(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestLnInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runLn("/nonexistent/ln.json", []string{"ln", "target"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runLn(bad spec) returned exit code %d, want 1", code)
	}
}
