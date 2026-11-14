// Package irc_framing provides a stateful byte-stream-to-line-frame converter.
//
// TCP delivers a byte stream, not messages. A single Read() call may return
// half a message, one full message, or three messages stitched together.
//
// IRC solves this with CRLF termination: every message ends with \r\n.
// The framer absorbs raw byte chunks and emits complete, \r\n-stripped lines.
//
// RFC 1459 maximum: 512 bytes per message including CRLF, so 510 bytes of content.
// RFC 2812 section 2.3 keeps the same 512-byte limit.
// Lines exceeding this limit are silently discarded.  A line that still has no
// LF after 511 bytes (510 plus a CR) is dropped early and the framer skips to
// the next LF, so a peer that never sends LF cannot make the buffer grow
// (SEC-011, CWE-400/770).
package irc_framing

import "bytes"

const Version = "0.1.0"

// maxContentBytes: RFC 1459 section 2.3 / RFC 2812 section 2.3 allow 512 bytes
// per message *including* the trailing CRLF, so at most 510 bytes of content.
const maxContentBytes = 510

// maxPendingBytes is the most we will hold for one line that has not yet seen
// its LF.  A legal line can be 510 content bytes plus a CR whose LF has not
// arrived yet, i.e. 511 bytes.  One more byte with no LF and the line can no
// longer be legal (SEC-011), so we stop remembering it.
const maxPendingBytes = maxContentBytes + 1

// Framer is a stateful byte-stream-to-line-frame converter.
// Call Feed with raw bytes. Call Frames to get complete CRLF-stripped lines.
// Not goroutine-safe — each connection should own its own Framer.
type Framer struct {
	buf []byte
	// tailLen is the length of the unterminated tail of buf (the bytes after
	// the last LF).  Invariant: tailLen <= maxPendingBytes after every Feed.
	tailLen int
	// discarding is true while we skip the rest of an overlong line:
	// everything up to and including the next LF is thrown away, unread.
	discarding bool
}

// NewFramer creates a new Framer with an empty buffer.
func NewFramer() *Framer {
	return &Framer{buf: make([]byte, 0, 1024)}
}

// Feed appends data to the internal buffer. Safe no-op for empty slices.
//
// Memory bound (SEC-011): the unterminated tail (bytes after the last LF)
// never exceeds maxPendingBytes = 511.  When it would, the partial line is
// dropped and the framer discards input up to the next LF -- fail closed, the
// same silent discard that already applies to a complete overlong line, so the
// API is unchanged.  Complete lines already buffered are kept; the caller
// bounds those by calling Frames after each read.
//
//	data = [ ...complete lines... LF ][ unterminated tail ]
//	                                  ^ only this part can be flooded
func (f *Framer) Feed(data []byte) {
	// Step 1: while discarding an overlong line, skip to just past its LF.
	if f.discarding {
		pos := bytes.IndexByte(data, '\n')
		if pos < 0 {
			return // still inside the overlong line
		}
		f.discarding = false
		data = data[pos+1:]
	}

	// Step 2: split data at its last LF.  The head (through that LF)
	// completes lines and is stored; only the tail can be flooded.
	head, tail := data[:0], data
	if pos := bytes.LastIndexByte(data, '\n'); pos >= 0 {
		head, tail = data[:pos+1], data[pos+1:]
	}
	if len(head) > 0 {
		f.buf = append(f.buf, head...)
		f.tailLen = 0
	}

	// Step 3: store the tail only if the pending line still fits.  We check
	// *before* appending so a huge single feed is never copied.
	if f.tailLen+len(tail) > maxPendingBytes {
		f.buf = f.buf[:len(f.buf)-f.tailLen] // drop the partial line
		f.tailLen = 0
		f.discarding = true
	} else {
		f.buf = append(f.buf, tail...)
		f.tailLen += len(tail)
	}
}

// Frames returns all complete lines from the buffer, with \r\n stripped.
// Lines exceeding 510 bytes of content are silently discarded.
func (f *Framer) Frames() [][]byte {
	var results [][]byte
	for {
		lfPos := indexByte(f.buf, '\n')
		if lfPos == -1 {
			break
		}
		contentEnd := lfPos
		if lfPos > 0 && f.buf[lfPos-1] == '\r' {
			contentEnd = lfPos - 1
		}
		line := make([]byte, contentEnd)
		copy(line, f.buf[:contentEnd])
		f.buf = f.buf[lfPos+1:]
		if len(line) > maxContentBytes {
			continue
		}
		results = append(results, line)
	}
	// Only the unterminated tail remains in the buffer.
	f.tailLen = len(f.buf)
	return results
}

// Reset discards all buffered data.
func (f *Framer) Reset() {
	f.buf = make([]byte, 0, 1024)
	f.tailLen = 0
	f.discarding = false
}

// BufferSize returns the number of bytes in the internal buffer.
func (f *Framer) BufferSize() int {
	return len(f.buf)
}

func indexByte(s []byte, b byte) int {
	for i, c := range s {
		if c == b {
			return i
		}
	}
	return -1
}
