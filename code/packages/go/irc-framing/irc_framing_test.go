package irc_framing

import (
	"bytes"
	"strings"
	"testing"
)

func TestNewFramer_Empty(t *testing.T) {
	f := NewFramer()
	if f.BufferSize() != 0 { t.Error("expected empty buffer") }
	if len(f.Frames()) != 0 { t.Error("expected no frames") }
}

func TestFeed_EmptySlice(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte{})
	if f.BufferSize() != 0 { t.Error("expected empty buffer") }
}

func TestFrames_SingleCRLF(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("NICK alice\r\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 frame, got %d", len(frames)) }
	if !bytes.Equal(frames[0], []byte("NICK alice")) { t.Errorf("got %q", frames[0]) }
	if f.BufferSize() != 0 { t.Error("expected empty buffer") }
}

func TestFrames_LFOnly(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("NICK bob\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 frame, got %d", len(frames)) }
	if !bytes.Equal(frames[0], []byte("NICK bob")) { t.Errorf("got %q", frames[0]) }
}

func TestFrames_MultipleMessages(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("NICK alice\r\nUSER alice 0 * :Alice\r\nJOIN #g\r\n"))
	frames := f.Frames()
	if len(frames) != 3 { t.Fatalf("expected 3 frames, got %d", len(frames)) }
	expected := []string{"NICK alice", "USER alice 0 * :Alice", "JOIN #g"}
	for i, want := range expected {
		if !bytes.Equal(frames[i], []byte(want)) {
			t.Errorf("frame[%d]: got %q want %q", i, frames[i], want)
		}
	}
}

func TestFrames_PartialMessage(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("NICK ali"))
	if len(f.Frames()) != 0 { t.Error("expected no frames from partial data") }
	if f.BufferSize() != 8 { t.Errorf("expected 8 bytes, got %d", f.BufferSize()) }
}

func TestFrames_PartialThenComplete(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("NICK ali"))
	if len(f.Frames()) != 0 { t.Error("expected no frames yet") }
	f.Feed([]byte("ce\r\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 frame, got %d", len(frames)) }
	if !bytes.Equal(frames[0], []byte("NICK alice")) { t.Errorf("got %q", frames[0]) }
}

func TestFrames_OverlongLineDiscarded(t *testing.T) {
	f := NewFramer()
	longContent := strings.Repeat("X", maxContentBytes+1)
	f.Feed([]byte(longContent + "\r\n"))
	frames := f.Frames()
	if len(frames) != 0 { t.Errorf("overlong line should be discarded, got %d frames", len(frames)) }
}

func TestFrames_ExactlyMaxLength(t *testing.T) {
	f := NewFramer()
	content := strings.Repeat("A", maxContentBytes)
	f.Feed([]byte(content + "\r\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 frame, got %d", len(frames)) }
	if len(frames[0]) != maxContentBytes { t.Errorf("expected %d bytes, got %d", maxContentBytes, len(frames[0])) }
}

func TestFrames_OverlongThenNormal(t *testing.T) {
	f := NewFramer()
	longContent := strings.Repeat("X", maxContentBytes+1)
	f.Feed([]byte(longContent + "\r\nPING :s\r\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 frame (PING), got %d", len(frames)) }
	if !bytes.Equal(frames[0], []byte("PING :s")) { t.Errorf("got %q", frames[0]) }
}

func TestReset_ClearsBuffer(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("partial data"))
	if f.BufferSize() == 0 { t.Fatal("expected non-empty buffer") }
	f.Reset()
	if f.BufferSize() != 0 { t.Errorf("expected empty after reset, got %d", f.BufferSize()) }
}

func TestReset_AllowsCleanReuse(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("NICK corrupted"))
	f.Reset()
	f.Feed([]byte("NICK fresh\r\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 frame, got %d", len(frames)) }
	if !bytes.Equal(frames[0], []byte("NICK fresh")) { t.Errorf("got %q", frames[0]) }
}

func TestFrames_BareNewline(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 empty frame, got %d", len(frames)) }
	if len(frames[0]) != 0 { t.Errorf("expected empty frame, got %q", frames[0]) }
}

func TestFrames_BareCRLF(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("\r\n"))
	frames := f.Frames()
	if len(frames) != 1 { t.Fatalf("expected 1 empty frame, got %d", len(frames)) }
	if len(frames[0]) != 0 { t.Errorf("expected empty frame, got %q", frames[0]) }
}

func TestFrames_DoubleDrain(t *testing.T) {
	f := NewFramer()
	f.Feed([]byte("PING :s\r\n"))
	if len(f.Frames()) != 1 { t.Fatal("expected 1 frame on first call") }
	if len(f.Frames()) != 0 { t.Fatal("expected 0 frames on second call") }
}

func TestBufferSize(t *testing.T) {
	f := NewFramer()
	data := []byte("partial")
	f.Feed(data)
	if f.BufferSize() != len(data) { t.Errorf("expected %d, got %d", len(data), f.BufferSize()) }
}

// ---------------------------------------------------------------------------
// SEC-011: a peer that never sends LF must not grow the buffer.
//
// The cap is 511 bytes for an unterminated line: 510 content bytes plus a CR
// whose LF has not arrived yet (RFC 1459 / RFC 2812 section 2.3: 512 bytes
// including CRLF).  The tests mirror the Rust crate's SEC-011 tests.
// ---------------------------------------------------------------------------

const sixteenMiB = 16 * 1024 * 1024

func TestSEC011_NoLFInChunksIsBounded(t *testing.T) {
	f := NewFramer()
	chunk := bytes.Repeat([]byte("A"), 4096)
	worst := 0
	for i := 0; i < sixteenMiB/len(chunk); i++ {
		f.Feed(chunk)
		if f.BufferSize() > worst { worst = f.BufferSize() }
	}
	if worst > 512 { t.Errorf("buffer grew to %d bytes with no LF", worst) }
	if len(f.Frames()) != 0 { t.Error("expected no frames") }
}

func TestSEC011_NoLFInOneFeedIsBounded(t *testing.T) {
	f := NewFramer()
	f.Feed(bytes.Repeat([]byte("A"), sixteenMiB))
	if f.BufferSize() > 512 { t.Errorf("buffer grew to %d", f.BufferSize()) }
	if len(f.Frames()) != 0 { t.Error("expected no frames") }
}

func TestSEC011_RecoversAfterFlood(t *testing.T) {
	// The flooded line is dropped whole, including bytes that arrive after
	// the cap, up to its LF; the next line frames normally.
	f := NewFramer()
	f.Feed(bytes.Repeat([]byte("A"), 4096))
	f.Feed(bytes.Repeat([]byte("B"), 4096))
	f.Feed([]byte("tail\r\nNICK alice\r\n"))
	frames := f.Frames()
	if len(frames) != 1 || string(frames[0]) != "NICK alice" {
		t.Fatalf("got %q", frames)
	}
	if f.BufferSize() != 0 { t.Errorf("buffer size %d", f.BufferSize()) }
}

func TestSEC011_PendingLineAtLimitIsKept(t *testing.T) {
	// 510 content bytes plus a CR is a legal line still waiting for its LF.
	f := NewFramer()
	f.Feed(append(bytes.Repeat([]byte("A"), 510), '\r'))
	if f.BufferSize() != 511 { t.Fatalf("buffer size %d, want 511", f.BufferSize()) }
	f.Feed([]byte("\n"))
	frames := f.Frames()
	if len(frames) != 1 || len(frames[0]) != 510 { t.Fatalf("got %d frames", len(frames)) }
}

func TestSEC011_PendingLineOverLimitIsDropped(t *testing.T) {
	f := NewFramer()
	f.Feed(bytes.Repeat([]byte("A"), 512))
	if f.BufferSize() != 0 { t.Fatalf("buffer size %d, want 0", f.BufferSize()) }
	f.Feed([]byte("\r\nPING x\r\n"))
	frames := f.Frames()
	if len(frames) != 1 || string(frames[0]) != "PING x" { t.Fatalf("got %q", frames) }
}

func TestSEC011_CompleteLinesBeforeFloodSurvive(t *testing.T) {
	f := NewFramer()
	f.Feed(append([]byte("NICK a\r\n"), bytes.Repeat([]byte("A"), 2000)...))
	frames := f.Frames()
	if len(frames) != 1 || string(frames[0]) != "NICK a" { t.Fatalf("got %q", frames) }
	if f.BufferSize() != 0 { t.Errorf("buffer size %d", f.BufferSize()) }
}

func TestSEC011_ResetClearsDiscardState(t *testing.T) {
	// After a flood the framer is skipping to the next LF.  Reset must end
	// that, or the first line of the next connection would be swallowed.
	f := NewFramer()
	f.Feed(bytes.Repeat([]byte("A"), 600))
	f.Reset()
	f.Feed([]byte("NICK a\r\n"))
	frames := f.Frames()
	if len(frames) != 1 || string(frames[0]) != "NICK a" { t.Fatalf("got %q", frames) }
}
