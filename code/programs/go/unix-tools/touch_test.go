// =========================================================================
// touch — Tests
// =========================================================================
//
// These tests verify the touch tool's behavior, covering:
//
//   1. Spec loading
//   2. Creating a new file
//   3. Updating timestamps of an existing file
//   4. No-create mode (-c)
//   5. Help and version flags

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestTouchSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "touch"), []string{"touch", "test"})
	if err != nil {
		t.Fatalf("failed to load touch.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

func TestTouchCreateNewFile(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "newfile.txt")

	var stdout, stderr bytes.Buffer
	code := runTouch(toolSpecPath(t, "touch"), []string{"touch", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTouch() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	if _, err := os.Stat(target); err != nil {
		t.Fatalf("file was not created: %v", err)
	}
}

func TestTouchUpdateExisting(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "existing.txt")
	os.WriteFile(target, []byte("data"), 0644)

	// Set old timestamp.
	oldTime := time.Date(2020, 1, 1, 0, 0, 0, 0, time.UTC)
	os.Chtimes(target, oldTime, oldTime)

	var stdout, stderr bytes.Buffer
	code := runTouch(toolSpecPath(t, "touch"), []string{"touch", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTouch(existing) returned exit code %d, want 0", code)
	}

	info, err := os.Stat(target)
	if err != nil {
		t.Fatalf("stat failed: %v", err)
	}

	// Modification time should be updated (more recent than oldTime).
	if !info.ModTime().After(oldTime) {
		t.Error("modification time was not updated")
	}
}

func TestTouchNoCreate(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "nonexistent.txt")

	var stdout, stderr bytes.Buffer
	code := runTouch(toolSpecPath(t, "touch"), []string{"touch", "-c", target}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTouch(-c) returned exit code %d, want 0", code)
	}

	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Error("file should not have been created with -c flag")
	}
}

func TestTouchMultipleFiles(t *testing.T) {
	dir := t.TempDir()
	f1 := filepath.Join(dir, "file1.txt")
	f2 := filepath.Join(dir, "file2.txt")

	var stdout, stderr bytes.Buffer
	code := runTouch(toolSpecPath(t, "touch"), []string{"touch", f1, f2}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTouch(multiple) returned exit code %d, want 0", code)
	}

	if _, err := os.Stat(f1); err != nil {
		t.Errorf("file1 was not created: %v", err)
	}
	if _, err := os.Stat(f2); err != nil {
		t.Errorf("file2 was not created: %v", err)
	}
}

func TestTouchHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTouch(toolSpecPath(t, "touch"), []string{"touch", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTouch(--help) returned exit code %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runTouch(--help) produced no stdout output")
	}
}

func TestTouchVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTouch(toolSpecPath(t, "touch"), []string{"touch", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTouch(--version) returned exit code %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runTouch(--version) output = %q, want %q", output, "1.0.0")
	}
}

func TestTouchInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTouch("/nonexistent/touch.json", []string{"touch", "test"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runTouch(bad spec) returned exit code %d, want 1", code)
	}
}
