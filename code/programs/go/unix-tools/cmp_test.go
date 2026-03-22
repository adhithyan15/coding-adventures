// =========================================================================
// cmp — Tests
// =========================================================================
//
// These tests verify the cmp tool's behavior, covering:
//
//   1. Spec loading
//   2. Identical files (exit code 0)
//   3. Different files — default output
//   4. Verbose mode (-l) — all differences
//   5. Silent mode (-s) — exit code only
//   6. Print bytes mode (-b)
//   7. Skip initial bytes (-i)
//   8. Max bytes limit (-n)
//   9. EOF on one file
//  10. Error handling (missing files)

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
// Spec loading test
// =========================================================================

func TestCmpSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "cmp"), []string{"cmp", "a", "b"})
	if err != nil {
		t.Fatalf("failed to load cmp.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Identical files
// =========================================================================

func TestCmpIdenticalFiles(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("hello world"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("hello world"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 for identical files", rc)
	}
	if stdout.Len() != 0 {
		t.Errorf("stdout should be empty for identical files, got %q", stdout.String())
	}
}

// =========================================================================
// Different files — default output
// =========================================================================

func TestCmpDifferentFiles(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("hello"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("hxllo"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1 for different files", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "differ") {
		t.Errorf("output should contain 'differ', got %q", output)
	}
	if !strings.Contains(output, "byte 2") {
		t.Errorf("output should report byte 2, got %q", output)
	}
}

// =========================================================================
// Verbose mode (-l)
// =========================================================================

func TestCmpVerboseMode(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("abc"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("axc"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", "-l",
			filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	// Should show byte number and octal values.
	if !strings.Contains(output, "2") {
		t.Errorf("verbose output should contain byte number 2, got %q", output)
	}
}

// =========================================================================
// Silent mode (-s)
// =========================================================================

func TestCmpSilentModeDifferent(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("hello"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("world"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", "-s",
			filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
	if stdout.Len() != 0 {
		t.Errorf("silent mode should produce no stdout, got %q", stdout.String())
	}
	if stderr.Len() != 0 {
		t.Errorf("silent mode should produce no stderr, got %q", stderr.String())
	}
}

func TestCmpSilentModeIdentical(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("same"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("same"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", "-s",
			filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
}

// =========================================================================
// Print bytes mode (-b)
// =========================================================================

func TestCmpPrintBytes(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("Ab"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("Xb"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", "-b",
			filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "A") || !strings.Contains(output, "X") {
		t.Errorf("print-bytes mode should show characters, got %q", output)
	}
}

// =========================================================================
// EOF on one file
// =========================================================================

func TestCmpEOFOnShorterFile(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "short.bin"), []byte("hi"), 0644)
	os.WriteFile(filepath.Join(dir, "long.bin"), []byte("hi there"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", filepath.Join(dir, "short.bin"), filepath.Join(dir, "long.bin")},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	if !strings.Contains(stderr.String(), "EOF") {
		t.Errorf("should report EOF, got stderr: %q", stderr.String())
	}
}

// =========================================================================
// Max bytes limit (-n)
// =========================================================================

func TestCmpMaxBytes(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("hello world"), 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte("hello earth"), 0644)

	var stdout, stderr bytes.Buffer
	// Only compare first 5 bytes — "hello" matches.
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", "-n", "5",
			filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin")},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0 (first 5 bytes identical)", rc)
	}
}

// =========================================================================
// Error handling
// =========================================================================

func TestCmpMissingFile(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runCmp(toolSpecPath(t, "cmp"),
		[]string{"cmp", filepath.Join(dir, "a.bin"), filepath.Join(dir, "missing.bin")},
		&stdout, &stderr)

	if rc != 2 {
		t.Errorf("exit code = %d, want 2 for error", rc)
	}
}

func TestCmpInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runCmp("/nonexistent/cmp.json", []string{"cmp", "a", "b"}, &stdout, &stderr)
	if rc != 2 {
		t.Errorf("exit code = %d, want 2", rc)
	}
}

// =========================================================================
// cmpFiles direct tests
// =========================================================================

func TestCmpFilesMultipleDiffsVerbose(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte{0x41, 0x42, 0x43}, 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte{0x41, 0x58, 0x59}, 0644)

	var stdout, stderr bytes.Buffer
	rc := cmpFiles(filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin"),
		CmpOptions{List: true}, &stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	lines := strings.Split(strings.TrimSpace(stdout.String()), "\n")
	if len(lines) != 2 {
		t.Errorf("expected 2 difference lines, got %d: %v", len(lines), lines)
	}
}

func TestCmpFilesVerboseWithPrintBytes(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "a.bin"), []byte{0x41, 0x42}, 0644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte{0x41, 0x58}, 0644)

	var stdout, stderr bytes.Buffer
	rc := cmpFiles(filepath.Join(dir, "a.bin"), filepath.Join(dir, "b.bin"),
		CmpOptions{List: true, PrintByte: true}, &stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "B") || !strings.Contains(output, "X") {
		t.Errorf("should show character representation, got %q", output)
	}
}
