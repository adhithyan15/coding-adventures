// =========================================================================
// paste — Tests
// =========================================================================

package main

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// =========================================================================
// pasteFiles tests
// =========================================================================

// TestPasteParallel verifies default (parallel) paste of two readers.
func TestPasteParallel(t *testing.T) {
	r1 := strings.NewReader("Alice\nBob\nCarol\n")
	r2 := strings.NewReader("25\n30\n22\n")

	got := pasteFiles([]io.Reader{r1, r2}, "\t", false)

	want := []string{"Alice\t25", "Bob\t30", "Carol\t22"}
	if len(got) != len(want) {
		t.Fatalf("pasteFiles length = %d, want %d", len(got), len(want))
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("pasteFiles[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestPasteParallelUnevenFiles verifies paste when files have different lengths.
func TestPasteParallelUnevenFiles(t *testing.T) {
	r1 := strings.NewReader("A\nB\nC\n")
	r2 := strings.NewReader("1\n")

	got := pasteFiles([]io.Reader{r1, r2}, "\t", false)

	// After r2 runs out, its columns should be empty.
	if len(got) < 1 {
		t.Fatal("expected at least 1 line")
	}
	if got[0] != "A\t1" {
		t.Errorf("line 0 = %q, want %q", got[0], "A\t1")
	}
}

// TestPasteSerial verifies serial mode.
func TestPasteSerial(t *testing.T) {
	r1 := strings.NewReader("A\nB\nC\n")
	r2 := strings.NewReader("1\n2\n3\n")

	got := pasteFiles([]io.Reader{r1, r2}, "\t", true)

	want := []string{"A\tB\tC", "1\t2\t3"}
	if len(got) != len(want) {
		t.Fatalf("pasteFiles(-s) length = %d, want %d. got: %v", len(got), len(want), got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("pasteFiles(-s)[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

// TestPasteCustomDelimiter verifies custom delimiter.
func TestPasteCustomDelimiter(t *testing.T) {
	r1 := strings.NewReader("A\nB\n")
	r2 := strings.NewReader("1\n2\n")

	got := pasteFiles([]io.Reader{r1, r2}, ",", false)

	if len(got) < 2 {
		t.Fatalf("expected 2 lines, got %d", len(got))
	}
	if got[0] != "A,1" {
		t.Errorf("line 0 = %q, want %q", got[0], "A,1")
	}
}

// TestPasteEmptyInput verifies handling of empty input.
func TestPasteEmptyInput(t *testing.T) {
	r1 := strings.NewReader("")
	got := pasteFiles([]io.Reader{r1}, "\t", false)
	if len(got) != 0 {
		t.Errorf("pasteFiles(empty) = %v, want empty", got)
	}
}

// =========================================================================
// runPaste integration tests
// =========================================================================

// TestRunPasteFiles verifies pasting two files.
func TestRunPasteFiles(t *testing.T) {
	dir := t.TempDir()
	f1 := filepath.Join(dir, "names.txt")
	f2 := filepath.Join(dir, "ages.txt")
	os.WriteFile(f1, []byte("Alice\nBob\n"), 0644)
	os.WriteFile(f2, []byte("25\n30\n"), 0644)

	var stdout, stderr bytes.Buffer
	code := runPasteWithStdin(
		toolSpecPath(t, "paste"),
		[]string{"paste", f1, f2},
		&stdout, &stderr, strings.NewReader(""),
	)

	if code != 0 {
		t.Errorf("runPaste returned %d, want 0. stderr: %s", code, stderr.String())
	}

	got := strings.TrimSpace(stdout.String())
	want := "Alice\t25\nBob\t30"
	if got != want {
		t.Errorf("runPaste output = %q, want %q", got, want)
	}
}

// TestRunPasteHelp verifies --help flag.
func TestRunPasteHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runPasteWithStdin(
		toolSpecPath(t, "paste"),
		[]string{"paste", "--help"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runPaste(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runPaste(--help) produced no output")
	}
}

// TestRunPasteVersion verifies --version flag.
func TestRunPasteVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runPasteWithStdin(
		toolSpecPath(t, "paste"),
		[]string{"paste", "--version"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 0 {
		t.Errorf("runPaste(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runPaste(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunPasteInvalidSpec verifies error handling.
func TestRunPasteInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runPasteWithStdin(
		"/nonexistent/paste.json",
		[]string{"paste", "-"},
		&stdout, &stderr, strings.NewReader(""),
	)
	if code != 1 {
		t.Errorf("runPaste(bad spec) returned %d, want 1", code)
	}
}
