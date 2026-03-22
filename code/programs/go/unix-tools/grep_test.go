// =========================================================================
// grep — Tests
// =========================================================================
//
// These tests verify the grep tool's behavior, covering:
//
//   1. Spec loading
//   2. Basic pattern matching
//   3. Case-insensitive matching (-i)
//   4. Inverted matching (-v)
//   5. Line numbering (-n)
//   6. Count mode (-c)
//   7. Files-with-matches mode (-l)
//   8. Only-matching mode (-o)
//   9. Word matching (-w)
//  10. Line matching (-x)
//  11. Fixed strings (-F)
//  12. Context lines (-A, -B, -C)
//  13. Recursive search (-r)
//  14. Exit codes (0 for match, 1 for no match, 2 for error)
//  15. Help and version flags

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading tests
// =========================================================================

func TestGrepSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "grep"), []string{"grep", "pattern"})
	if err != nil {
		t.Fatalf("failed to load grep.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Business logic tests — compilePattern
// =========================================================================

func TestCompilePatternBasic(t *testing.T) {
	re, err := compilePattern("hello", GrepOptions{})
	if err != nil {
		t.Fatalf("compilePattern() failed: %v", err)
	}
	if !re.MatchString("say hello world") {
		t.Error("pattern should match 'say hello world'")
	}
}

func TestCompilePatternIgnoreCase(t *testing.T) {
	re, err := compilePattern("hello", GrepOptions{IgnoreCase: true})
	if err != nil {
		t.Fatalf("compilePattern(-i) failed: %v", err)
	}
	if !re.MatchString("HELLO World") {
		t.Error("case-insensitive pattern should match 'HELLO World'")
	}
}

func TestCompilePatternFixedStrings(t *testing.T) {
	re, err := compilePattern("a.b", GrepOptions{FixedStrings: true})
	if err != nil {
		t.Fatalf("compilePattern(-F) failed: %v", err)
	}
	if re.MatchString("acb") {
		t.Error("fixed string 'a.b' should NOT match 'acb'")
	}
	if !re.MatchString("a.b") {
		t.Error("fixed string 'a.b' should match 'a.b'")
	}
}

func TestCompilePatternWordRegexp(t *testing.T) {
	re, err := compilePattern("cat", GrepOptions{WordRegexp: true})
	if err != nil {
		t.Fatalf("compilePattern(-w) failed: %v", err)
	}
	if re.MatchString("concatenate") {
		t.Error("word regexp 'cat' should NOT match 'concatenate'")
	}
	if !re.MatchString("the cat sat") {
		t.Error("word regexp 'cat' should match 'the cat sat'")
	}
}

func TestCompilePatternLineRegexp(t *testing.T) {
	re, err := compilePattern("exact", GrepOptions{LineRegexp: true})
	if err != nil {
		t.Fatalf("compilePattern(-x) failed: %v", err)
	}
	if re.MatchString("not exact match") {
		t.Error("line regexp should NOT match partial line")
	}
	if !re.MatchString("exact") {
		t.Error("line regexp should match exact line")
	}
}

func TestCompilePatternInvalid(t *testing.T) {
	_, err := compilePattern("[invalid", GrepOptions{})
	if err == nil {
		t.Error("expected error for invalid regexp, got nil")
	}
}

// =========================================================================
// Business logic tests — grepLine
// =========================================================================

func TestGrepLineMatch(t *testing.T) {
	re := regexp.MustCompile("hello")
	if !grepLine("hello world", re, GrepOptions{}) {
		t.Error("'hello world' should match 'hello'")
	}
}

func TestGrepLineNoMatch(t *testing.T) {
	re := regexp.MustCompile("hello")
	if grepLine("goodbye world", re, GrepOptions{}) {
		t.Error("'goodbye world' should not match 'hello'")
	}
}

func TestGrepLineInvert(t *testing.T) {
	re := regexp.MustCompile("hello")

	// Matching line with invert: should return false.
	if grepLine("hello world", re, GrepOptions{InvertMatch: true}) {
		t.Error("inverted match: 'hello world' should return false")
	}

	// Non-matching line with invert: should return true.
	if !grepLine("goodbye world", re, GrepOptions{InvertMatch: true}) {
		t.Error("inverted match: 'goodbye world' should return true")
	}
}

// =========================================================================
// Business logic tests — grepFile
// =========================================================================

func TestGrepFileBasic(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("hello world\ngoodbye world\nhello again\n"), 0644)

	re := regexp.MustCompile("hello")
	matches, err := grepFile(file, re, GrepOptions{})
	if err != nil {
		t.Fatalf("grepFile() failed: %v", err)
	}

	matchCount := 0
	for _, m := range matches {
		if m.IsMatch {
			matchCount++
		}
	}

	if matchCount != 2 {
		t.Errorf("expected 2 matches, got %d", matchCount)
	}
}

func TestGrepFileWithContext(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("line1\nline2\nmatch\nline4\nline5\n"), 0644)

	re := regexp.MustCompile("match")
	matches, err := grepFile(file, re, GrepOptions{BeforeContext: 1, AfterContext: 1})
	if err != nil {
		t.Fatalf("grepFile() with context failed: %v", err)
	}

	// Should have: line2 (before), match, line4 (after) = 3 entries.
	if len(matches) != 3 {
		t.Errorf("expected 3 entries (1 before + match + 1 after), got %d", len(matches))
	}
}

func TestGrepFileMaxCount(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("match1\nmatch2\nmatch3\n"), 0644)

	re := regexp.MustCompile("match")
	matches, err := grepFile(file, re, GrepOptions{MaxCount: 2})
	if err != nil {
		t.Fatalf("grepFile() with max-count failed: %v", err)
	}

	matchCount := 0
	for _, m := range matches {
		if m.IsMatch {
			matchCount++
		}
	}

	if matchCount != 2 {
		t.Errorf("expected 2 matches with max-count=2, got %d", matchCount)
	}
}

func TestGrepFileNonexistent(t *testing.T) {
	re := regexp.MustCompile("test")
	_, err := grepFile("/nonexistent/file.txt", re, GrepOptions{})
	if err == nil {
		t.Error("expected error for nonexistent file, got nil")
	}
}

// =========================================================================
// runGrep integration tests
// =========================================================================

func TestRunGrepBasicMatch(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("hello world\ngoodbye world\nhello again\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "hello", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "hello world") {
		t.Errorf("output should contain 'hello world': %q", output)
	}
}

func TestRunGrepNoMatch(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("hello world\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "missing", file}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runGrep(no match) returned %d, want 1", code)
	}
}

func TestRunGrepCaseInsensitive(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("Hello World\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-i", "hello", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-i) returned %d, want 0", code)
	}
}

func TestRunGrepInvert(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("hello\nworld\nhello\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-v", "hello", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-v) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "world" {
		t.Errorf("inverted output = %q, want %q", output, "world")
	}
}

func TestRunGrepCount(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("hello\nworld\nhello\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-c", "hello", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-c) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "2" {
		t.Errorf("count output = %q, want %q", output, "2")
	}
}

func TestRunGrepFilesWithMatches(t *testing.T) {
	dir := t.TempDir()
	file1 := filepath.Join(dir, "a.txt")
	file2 := filepath.Join(dir, "b.txt")
	os.WriteFile(file1, []byte("hello\n"), 0644)
	os.WriteFile(file2, []byte("world\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-l", "hello", file1, file2}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-l) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != file1 {
		t.Errorf("files-with-matches output = %q, want %q", output, file1)
	}
}

func TestRunGrepLineNumber(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("aaa\nbbb\nccc\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-n", "bbb", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-n) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if !strings.Contains(output, "2:bbb") {
		t.Errorf("line number output should contain '2:bbb': %q", output)
	}
}

func TestRunGrepOnlyMatching(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("hello world\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-o", "hello", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-o) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "hello" {
		t.Errorf("only-matching output = %q, want %q", output, "hello")
	}
}

func TestRunGrepRecursive(t *testing.T) {
	dir := t.TempDir()
	subdir := filepath.Join(dir, "sub")
	os.Mkdir(subdir, 0755)
	os.WriteFile(filepath.Join(dir, "a.txt"), []byte("findme\n"), 0644)
	os.WriteFile(filepath.Join(subdir, "b.txt"), []byte("findme too\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-r", "findme", dir}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-r) returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "findme") {
		t.Errorf("recursive output should contain 'findme': %q", output)
	}
}

func TestRunGrepFixedStrings(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("a.b\nacb\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "-F", "a.b", file}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(-F) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "a.b" {
		t.Errorf("fixed-strings output = %q, want %q", output, "a.b")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

func TestGrepHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runGrep(--help) produced no output")
	}
}

func TestGrepVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runGrep(--version) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runGrep(--version) = %q, want %q", output, "1.0.0")
	}
}

func TestGrepInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runGrep("/nonexistent/grep.json", []string{"grep", "test"}, &stdout, &stderr)

	if code != 2 {
		t.Errorf("runGrep(bad spec) returned %d, want 2", code)
	}
}

func TestGrepInvalidPattern(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runGrep(toolSpecPath(t, "grep"), []string{"grep", "[invalid", file}, &stdout, &stderr)

	if code != 2 {
		t.Errorf("runGrep(bad pattern) returned %d, want 2", code)
	}
}
