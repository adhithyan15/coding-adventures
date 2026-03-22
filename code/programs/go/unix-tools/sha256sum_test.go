// =========================================================================
// sha256sum — Tests
// =========================================================================

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// computeSHA256 tests
// =========================================================================

// TestComputeSHA256Empty verifies the SHA-256 of empty input.
func TestComputeSHA256Empty(t *testing.T) {
	hash, err := computeSHA256(strings.NewReader(""))
	if err != nil {
		t.Fatalf("computeSHA256(\"\") failed: %v", err)
	}
	// SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
	want := "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
	if hash != want {
		t.Errorf("computeSHA256(\"\") = %q, want %q", hash, want)
	}
}

// TestComputeSHA256KnownInput verifies SHA-256 of a known string.
func TestComputeSHA256KnownInput(t *testing.T) {
	hash, err := computeSHA256(strings.NewReader("hello\n"))
	if err != nil {
		t.Fatalf("computeSHA256(\"hello\\n\") failed: %v", err)
	}
	// SHA-256("hello\n") = 5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03
	want := "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
	if hash != want {
		t.Errorf("computeSHA256(\"hello\\n\") = %q, want %q", hash, want)
	}
}

// TestComputeSHA256DifferentInputs verifies that different inputs produce different hashes.
func TestComputeSHA256DifferentInputs(t *testing.T) {
	hash1, _ := computeSHA256(strings.NewReader("foo"))
	hash2, _ := computeSHA256(strings.NewReader("bar"))
	if hash1 == hash2 {
		t.Errorf("different inputs produced same hash: %q", hash1)
	}
}

// TestComputeSHA256Length verifies that SHA-256 hashes are 64 hex chars.
func TestComputeSHA256Length(t *testing.T) {
	hash, err := computeSHA256(strings.NewReader("test"))
	if err != nil {
		t.Fatalf("computeSHA256 failed: %v", err)
	}
	if len(hash) != 64 {
		t.Errorf("SHA-256 hash length = %d, want 64", len(hash))
	}
}

// =========================================================================
// checkSHA256 tests
// =========================================================================

// TestCheckSHA256 verifies the check mode with a real file.
func TestCheckSHA256(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello\n"), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	checksumContent := "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03  " + testFile + "\n"
	results, err := checkSHA256(strings.NewReader(checksumContent))
	if err != nil {
		t.Fatalf("checkSHA256 failed: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	if !results[0].OK {
		t.Errorf("checkSHA256: expected OK for valid hash")
	}
}

// TestCheckSHA256Mismatch verifies that a wrong hash is detected.
func TestCheckSHA256Mismatch(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello\n"), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	checksumContent := "0000000000000000000000000000000000000000000000000000000000000000  " + testFile + "\n"
	results, err := checkSHA256(strings.NewReader(checksumContent))
	if err != nil {
		t.Fatalf("checkSHA256 failed: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	if results[0].OK {
		t.Errorf("checkSHA256: expected FAILED for wrong hash")
	}
}

// =========================================================================
// runSha256sum integration tests
// =========================================================================

// TestRunSha256sumFile verifies hashing a file.
func TestRunSha256sumFile(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello\n"), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	var stdout, stderr bytes.Buffer
	code := runSha256sumWithStdin(
		toolSpecPath(t, "sha256sum"),
		[]string{"sha256sum", testFile},
		&stdout, &stderr, strings.NewReader(""),
	)

	if code != 0 {
		t.Errorf("runSha256sum returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03") {
		t.Errorf("output missing expected hash: %q", output)
	}
}

// TestRunSha256sumStdin verifies hashing stdin.
func TestRunSha256sumStdin(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSha256sumWithStdin(
		toolSpecPath(t, "sha256sum"),
		[]string{"sha256sum"},
		&stdout, &stderr, strings.NewReader("hello\n"),
	)

	if code != 0 {
		t.Errorf("runSha256sum(stdin) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03") {
		t.Errorf("output missing expected hash: %q", output)
	}
}

// TestRunSha256sumHelp verifies --help flag.
func TestRunSha256sumHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSha256sumWithStdin(
		toolSpecPath(t, "sha256sum"),
		[]string{"sha256sum", "--help"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runSha256sum(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runSha256sum(--help) produced no output")
	}
}

// TestRunSha256sumVersion verifies --version flag.
func TestRunSha256sumVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSha256sumWithStdin(
		toolSpecPath(t, "sha256sum"),
		[]string{"sha256sum", "--version"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runSha256sum(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runSha256sum(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunSha256sumInvalidSpec verifies error handling.
func TestRunSha256sumInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runSha256sumWithStdin(
		"/nonexistent/sha256sum.json",
		[]string{"sha256sum"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runSha256sum(bad spec) returned %d, want 1", code)
	}
}
