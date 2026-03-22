// =========================================================================
// md5sum — Tests
// =========================================================================
//
// These tests verify the md5sum tool's behavior:
//   1. computeMD5: hash computation on known inputs
//   2. parseChecksumLine: line parsing
//   3. checkMD5: verification mode
//   4. runMd5sum: integration tests

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// computeMD5 tests
// =========================================================================

// TestComputeMD5Empty verifies the MD5 of empty input.
// The MD5 of empty input is a well-known constant.
func TestComputeMD5Empty(t *testing.T) {
	hash, err := computeMD5(strings.NewReader(""))
	if err != nil {
		t.Fatalf("computeMD5(\"\") failed: %v", err)
	}
	// MD5("") = d41d8cd98f00b204e9800998ecf8427e
	if hash != "d41d8cd98f00b204e9800998ecf8427e" {
		t.Errorf("computeMD5(\"\") = %q, want d41d8cd98f00b204e9800998ecf8427e", hash)
	}
}

// TestComputeMD5KnownInput verifies MD5 of a known string.
func TestComputeMD5KnownInput(t *testing.T) {
	hash, err := computeMD5(strings.NewReader("hello\n"))
	if err != nil {
		t.Fatalf("computeMD5(\"hello\\n\") failed: %v", err)
	}
	// MD5("hello\n") = b1946ac92492d2347c6235b4d2611184
	if hash != "b1946ac92492d2347c6235b4d2611184" {
		t.Errorf("computeMD5(\"hello\\n\") = %q, want b1946ac92492d2347c6235b4d2611184", hash)
	}
}

// TestComputeMD5DifferentInputs verifies that different inputs produce different hashes.
func TestComputeMD5DifferentInputs(t *testing.T) {
	hash1, _ := computeMD5(strings.NewReader("foo"))
	hash2, _ := computeMD5(strings.NewReader("bar"))
	if hash1 == hash2 {
		t.Errorf("different inputs produced same hash: %q", hash1)
	}
}

// =========================================================================
// parseChecksumLine tests
// =========================================================================

// TestParseChecksumLineText verifies parsing text-mode checksum lines.
func TestParseChecksumLineText(t *testing.T) {
	hash, filename, err := parseChecksumLine("d41d8cd98f00b204e9800998ecf8427e  empty.txt")
	if err != nil {
		t.Fatalf("parseChecksumLine failed: %v", err)
	}
	if hash != "d41d8cd98f00b204e9800998ecf8427e" {
		t.Errorf("hash = %q, want d41d8cd98f00b204e9800998ecf8427e", hash)
	}
	if filename != "empty.txt" {
		t.Errorf("filename = %q, want empty.txt", filename)
	}
}

// TestParseChecksumLineBinary verifies parsing binary-mode checksum lines.
func TestParseChecksumLineBinary(t *testing.T) {
	hash, filename, err := parseChecksumLine("d41d8cd98f00b204e9800998ecf8427e *binary.dat")
	if err != nil {
		t.Fatalf("parseChecksumLine failed: %v", err)
	}
	if hash != "d41d8cd98f00b204e9800998ecf8427e" {
		t.Errorf("hash = %q", hash)
	}
	if filename != "binary.dat" {
		t.Errorf("filename = %q, want binary.dat", filename)
	}
}

// =========================================================================
// checkMD5 tests
// =========================================================================

// TestCheckMD5 verifies the check mode with a real file.
func TestCheckMD5(t *testing.T) {
	// Create a temp file with known content.
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello\n"), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	// Create a checksum file referencing the test file.
	checksumContent := "b1946ac92492d2347c6235b4d2611184  " + testFile + "\n"
	results, err := checkMD5(strings.NewReader(checksumContent))
	if err != nil {
		t.Fatalf("checkMD5 failed: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	if !results[0].OK {
		t.Errorf("checkMD5: expected OK for valid hash")
	}
}

// TestCheckMD5Mismatch verifies that a wrong hash is detected.
func TestCheckMD5Mismatch(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello\n"), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	// Wrong hash.
	checksumContent := "0000000000000000000000000000000  " + testFile + "\n"
	results, err := checkMD5(strings.NewReader(checksumContent))
	if err != nil {
		t.Fatalf("checkMD5 failed: %v", err)
	}

	if len(results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(results))
	}
	if results[0].OK {
		t.Errorf("checkMD5: expected FAILED for wrong hash")
	}
}

// =========================================================================
// runMd5sum integration tests
// =========================================================================

// TestRunMd5sumFile verifies hashing a file.
func TestRunMd5sumFile(t *testing.T) {
	dir := t.TempDir()
	testFile := filepath.Join(dir, "test.txt")
	if err := os.WriteFile(testFile, []byte("hello\n"), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	var stdout, stderr bytes.Buffer
	code := runMd5sumWithStdin(
		toolSpecPath(t, "md5sum"),
		[]string{"md5sum", testFile},
		&stdout, &stderr, strings.NewReader(""),
	)

	if code != 0 {
		t.Errorf("runMd5sum returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "b1946ac92492d2347c6235b4d2611184") {
		t.Errorf("output missing expected hash: %q", output)
	}
}

// TestRunMd5sumStdin verifies hashing stdin.
func TestRunMd5sumStdin(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMd5sumWithStdin(
		toolSpecPath(t, "md5sum"),
		[]string{"md5sum"},
		&stdout, &stderr, strings.NewReader("hello\n"),
	)

	if code != 0 {
		t.Errorf("runMd5sum(stdin) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "b1946ac92492d2347c6235b4d2611184") {
		t.Errorf("output missing expected hash: %q", output)
	}
}

// TestRunMd5sumHelp verifies --help flag.
func TestRunMd5sumHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMd5sumWithStdin(
		toolSpecPath(t, "md5sum"),
		[]string{"md5sum", "--help"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runMd5sum(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runMd5sum(--help) produced no output")
	}
}

// TestRunMd5sumVersion verifies --version flag.
func TestRunMd5sumVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMd5sumWithStdin(
		toolSpecPath(t, "md5sum"),
		[]string{"md5sum", "--version"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runMd5sum(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runMd5sum(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunMd5sumInvalidSpec verifies error handling for bad spec path.
func TestRunMd5sumInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMd5sumWithStdin(
		"/nonexistent/md5sum.json",
		[]string{"md5sum"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runMd5sum(bad spec) returned %d, want 1", code)
	}
}

// TestRunMd5sumNonexistentFile verifies error for missing file.
func TestRunMd5sumNonexistentFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runMd5sumWithStdin(
		toolSpecPath(t, "md5sum"),
		[]string{"md5sum", "/nonexistent/file.txt"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runMd5sum(missing file) returned %d, want 1", code)
	}
}
