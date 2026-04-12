// Package irc_framing provides a stateful byte-stream-to-line-frame converter.
//
// TCP delivers a byte stream, not messages. A single Read() call may return
// half a message, one full message, or three messages stitched together.
//
// IRC solves this with CRLF termination: every message ends with \r\n.
// The framer absorbs raw byte chunks and emits complete, \r\n-stripped lines.
//
// RFC 1459 maximum: 512 bytes per message including CRLF, so 510 bytes of content.
// Lines exceeding this limit are silently discarded.
package irc_framing

const Version = "0.1.0"
const maxContentBytes = 510

// Framer is a stateful byte-stream-to-line-frame converter.
// Call Feed with raw bytes. Call Frames to get complete CRLF-stripped lines.
// Not goroutine-safe — each connection should own its own Framer.
type Framer struct {
	buf []byte
}

// NewFramer creates a new Framer with an empty buffer.
func NewFramer() *Framer {
	return &Framer{buf: make([]byte, 0, 1024)}
}

// Feed appends data to the internal buffer. Safe no-op for empty slices.
func (f *Framer) Feed(data []byte) {
	f.buf = append(f.buf, data...)
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
	return results
}

// Reset discards all buffered data.
func (f *Framer) Reset() {
	f.buf = make([]byte, 0, 1024)
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
