// compiler_test.go — tests for cappedWriter (#13179)

package main

import (
	"strings"
	"testing"
)

func TestCappedWriter_UnderLimit(t *testing.T) {
	w := &cappedWriter{limit: 100}
	n, err := w.Write([]byte("hello"))
	if err != nil {
		t.Fatalf("Write returned error: %v", err)
	}
	if n != 5 {
		t.Errorf("Write returned n=%d, want 5", n)
	}
	if w.String() != "hello" {
		t.Errorf("String() = %q, want %q", w.String(), "hello")
	}
}

func TestCappedWriter_ExactlyAtLimit(t *testing.T) {
	w := &cappedWriter{limit: 5}
	if _, err := w.Write([]byte("hello")); err != nil {
		t.Fatalf("Write returned error: %v", err)
	}
	if w.String() != "hello" {
		t.Errorf("String() = %q, want %q (no truncation marker at exactly the limit)", w.String(), "hello")
	}
}

// The core fix: writes past the limit are silently discarded from the
// retained buffer rather than growing it — this is what bounds memory use
// as bytes arrive, instead of only checking size after the fact.
func TestCappedWriter_TruncatesAtLimit(t *testing.T) {
	w := &cappedWriter{limit: 5}
	n, err := w.Write([]byte("hello world"))
	if err != nil {
		t.Fatalf("Write returned error: %v", err)
	}
	// Write must report success for the FULL input, not just what it kept —
	// a short return would look like a write error to the caller (cmd.Run's
	// pipe-copying goroutine), which could kill the subprocess mid-stream.
	if n != len("hello world") {
		t.Errorf("Write returned n=%d, want %d (must report full length even though only part is retained)", n, len("hello world"))
	}
	if len(w.buf) != 5 {
		t.Errorf("buf grew to %d bytes, want capped at 5", len(w.buf))
	}
	got := w.String()
	if !strings.HasPrefix(got, "hello") {
		t.Errorf("String() = %q, want it to start with the retained prefix %q", got, "hello")
	}
	if !strings.Contains(got, "truncated") {
		t.Errorf("String() = %q, want a truncation marker since input exceeded the limit", got)
	}
}

// Multiple writes past the limit (the real shape cmd.Run's stdout/stderr
// pipe-copying produces — many small Write calls, not one big one) must not
// let the buffer creep past the cap one chunk at a time.
func TestCappedWriter_MultipleWritesStayCapped(t *testing.T) {
	w := &cappedWriter{limit: 10}
	for i := 0; i < 1000; i++ {
		if _, err := w.Write([]byte("0123456789")); err != nil {
			t.Fatalf("Write %d returned error: %v", i, err)
		}
	}
	if len(w.buf) != 10 {
		t.Errorf("buf = %d bytes after 1000 writes, want capped at 10", len(w.buf))
	}
	if w.written != 10000 {
		t.Errorf("written = %d, want 10000 (total offered, independent of what's retained)", w.written)
	}
}

func TestCappedWriter_EmptyNeverTruncated(t *testing.T) {
	w := &cappedWriter{limit: 10}
	if w.String() != "" {
		t.Errorf("String() on an unwritten cappedWriter = %q, want empty", w.String())
	}
}
