package exechead

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"hash"
	"io"
	"unicode/utf8"
)

// Limits are the file's caps; the zero value means the format's own.
type Limits struct {
	MaxLines int
	MaxBytes int64
}

func (l Limits) lines() int {
	if l.MaxLines > 0 {
		return l.MaxLines
	}
	return MaxLines
}

func (l Limits) bytes() int64 {
	if l.MaxBytes > 0 {
		return l.MaxBytes
	}
	return MaxFileBytes
}

// ErrOverflow is Append's answer when the line or byte cap is passed.  Nothing
// was written, and every later Append answers it too: the supervisor stops
// writing, marks the step unclean and goes on answering the kernel.
var ErrOverflow = errors.New("exechead: the sidecar's line or byte cap is passed")

// Writer encodes the file one line at a time: it numbers the lines, counts and
// hashes every byte it passes on, and enforces the caps.  It is not safe for
// concurrent use; the supervisor calls it under its own lock.
type Writer struct {
	w     io.Writer
	h     hash.Hash
	lim   Limits
	n     int64 // bytes written, all hashed
	lines int   // notification lines written
	err   error // sticky
}

// NewWriter writes the header to w and returns the writer.
func NewWriter(w io.Writer, h Header, lim Limits) (*Writer, error) {
	h.V = Version
	if err := checkHeader(h); err != nil {
		return nil, err
	}
	data, err := encodeCanonical(wireHeader{V: &h.V, Kernel: &h.Kernel, Nonce: &h.Nonce, ActionSHA256: &h.ActionSHA256, Step: &h.Step, WaitKillable: &h.WaitKillable})
	if err != nil {
		return nil, err
	}
	wr := &Writer{w: w, h: sha256.New(), lim: lim}
	if err := wr.put(data); err != nil {
		return nil, err
	}
	return wr, nil
}

// put writes one encoded line (newline added), counting and hashing what the
// destination took.  A short or failed write is sticky.
func (w *Writer) put(line []byte) error {
	buf := append(line[:len(line):len(line)], '\n')
	n, err := w.w.Write(buf)
	if n > 0 {
		w.h.Write(buf[:n])
		w.n += int64(n)
	}
	if err == nil && n < len(buf) {
		err = io.ErrShortWrite
	}
	if err != nil {
		w.err = fmt.Errorf("exechead: writing the sidecar: %w", err)
		return w.err
	}
	return nil
}

// Append numbers and writes one line.  It returns ErrOverflow past a cap (and
// writes nothing), a validation error for a line the decoder would refuse, or
// the write error.  A line that is refused does not use a sequence number.
func (w *Writer) Append(l Line) error {
	if w.err != nil {
		return w.err
	}
	l.Seq = w.lines
	data, err := EncodeLine(l)
	if err != nil {
		return err
	}
	if w.lines >= w.lim.lines() || w.n+int64(len(data))+1 > w.lim.bytes() {
		w.err = ErrOverflow
		return w.err
	}
	if err := w.put(data); err != nil {
		return err
	}
	w.lines++
	return nil
}

// Lines is the number of notification lines written.
func (w *Writer) Lines() int { return w.lines }

// Bytes is the size of the file so far.
func (w *Writer) Bytes() int64 { return w.n }

// SHA256 is the digest of the file so far, in lowercase hex.
func (w *Writer) SHA256() string { return hex.EncodeToString(w.h.Sum(nil)) }

// Err is the sticky error, if any.
func (w *Writer) Err() error { return w.err }

// Overflowed says a cap was passed (and not, say, a write failure).
func (w *Writer) Overflowed() bool { return errors.Is(w.err, ErrOverflow) }

// EncodeLine validates and renders one line: compact, no HTML escaping, no
// newline.  It refuses what the decoder would (a path that is not UTF-8 or is
// too long, a status outside the enum, a head on a line that is not confirmed).
func EncodeLine(l Line) ([]byte, error) {
	if err := checkLine(l); err != nil {
		return nil, err
	}
	st := string(l.Status)
	w := wireLine{Seq: &l.Seq, Tid: &l.Tid, Tgid: &l.Tgid, Syscall: &l.Syscall, Path: &l.Path, Status: &st}
	if l.Status == StatusConfirmed {
		head := base64.StdEncoding.EncodeToString(l.Head)
		w.Dev, w.Ino, w.Head, w.SHA256 = &l.Dev, &l.Ino, &head, &l.SHA256
	}
	data, err := encodeCanonical(w)
	if err != nil {
		return nil, err
	}
	if len(data) > MaxLineBytes {
		return nil, fmt.Errorf("exechead: a line of %d bytes is over the cap of %d", len(data), MaxLineBytes)
	}
	return data, nil
}

// checkLine is the semantic validation shared by the encoder and the decoder.
func checkLine(l Line) error {
	switch {
	case l.Seq < 0:
		return fmt.Errorf("seq %d", l.Seq)
	case l.Tid <= 0 || l.Tgid <= 0:
		return fmt.Errorf("tid %d, tgid %d", l.Tid, l.Tgid)
	case l.Syscall != SyscallExecve && l.Syscall != SyscallExecveat:
		return fmt.Errorf("syscall %q", clip(l.Syscall))
	case !l.Status.Valid():
		return fmt.Errorf("status %q is not one of the eleven", clip(string(l.Status)))
	case len(l.Path) >= PathMax:
		return fmt.Errorf("a path of %d bytes (the most is %d)", len(l.Path), PathMax-1)
	case !utf8.ValidString(l.Path):
		return errors.New("a path that is not UTF-8 (the supervisor records \"\" and status unpinned)")
	}
	if l.Status != StatusConfirmed {
		if l.Dev != 0 || l.Ino != 0 || l.Head != nil || l.SHA256 != "" {
			return fmt.Errorf("a %s line carries dev, ino, head or sha256", l.Status)
		}
		return nil
	}
	if len(l.Head) > HeadLimit {
		return fmt.Errorf("a head of %d bytes (the most is %d)", len(l.Head), HeadLimit)
	}
	sum := sha256.Sum256(l.Head)
	if l.SHA256 != hex.EncodeToString(sum[:]) {
		return errors.New("sha256 is not the SHA-256 of the head")
	}
	return nil
}

func checkHeader(h Header) error {
	switch {
	case h.V != Version:
		return fmt.Errorf("version %d, want %d", h.V, Version)
	case len(h.Kernel) > MaxKernelBytes || !printableASCII(h.Kernel):
		return errors.New("kernel is not short printable ASCII")
	case !hexN(h.Nonce, 32):
		return errors.New("nonce is not 32 lowercase hex digits")
	case !hexN(h.ActionSHA256, 64):
		return errors.New("action_sha256 is not a SHA-256 in lowercase hex")
	case h.Step < 0:
		return fmt.Errorf("step %d", h.Step)
	}
	return nil
}

func printableASCII(s string) bool {
	for i := 0; i < len(s); i++ {
		if s[i] < 0x20 || s[i] > 0x7e {
			return false
		}
	}
	return true
}

// clip shortens text that came from a file for an error message, and replaces
// every character outside printable ASCII.
func clip(s string) string {
	if len(s) > 40 {
		s = s[:40] + "..."
	}
	return string(bytes.Map(func(r rune) rune {
		if r < 0x20 || r > 0x7e {
			return '?'
		}
		return r
	}, []byte(s)))
}

// encodeCanonical marshals compactly without HTML escaping, newline trimmed.
func encodeCanonical(v any) ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(v); err != nil {
		return nil, err
	}
	return bytes.TrimSuffix(buf.Bytes(), []byte("\n")), nil
}
