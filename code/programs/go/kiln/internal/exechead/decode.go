package exechead

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// File is a decoded sidecar.
type File struct {
	Header Header
	Lines  []Line
	// Bytes and SHA256 describe the file as read: the numbers the step sidecar's
	// exechead_bytes and exechead_sha256 must equal.
	Bytes  int64
	SHA256 string
}

// Expect is what a caller already knows the file must say.  A zero field is not
// checked, except Step, which is checked only when CheckStep is set (zero is a
// valid step number).
type Expect struct {
	Nonce        string
	ActionSHA256 string
	Step         int
	CheckStep    bool
}

// Decode reads a sidecar strictly.  PR 5's classifier uses it, so it is as
// strict as the format allows:
//
//   - the file ends in a newline, is not empty, and no line is empty;
//   - no line is over MaxLineBytes, the file is under the byte cap and holds at
//     most the line cap of notification lines;
//   - every line is a JSON object with exactly its keys (an unknown key, a
//     duplicated key and a key whose case differs from the spelling are all
//     refused, and so is a missing one);
//   - the first line is the header, whose version, nonce and digest shapes hold,
//     and which names the expected step if the caller says so;
//   - `seq` is the line's index, so a gap, a repeat and a reorder all fail;
//   - a status is one of the eleven, and the four confirmed-only fields appear
//     on `confirmed` lines and on no other;
//   - a head is at most HeadLimit bytes and its sha256 is the head's digest;
//   - finally every line is RE-ENCODED and compared byte for byte with what was
//     read, so spacing, key order, escape spelling and base64 variants (none of
//     which the supervisor ever writes) are refused too.
//
// Duplicates and gaps are therefore detected by `seq`, never by comparing text.
// The per-line sha256 is self-attested: it detects a corrupted line, not a
// forged one.  The binding to the step is the whole file's digest in the step
// sidecar plus the nonce here.
func Decode(data []byte, lim Limits, want *Expect) (*File, error) {
	if len(data) == 0 {
		return nil, errors.New("exechead: the file is empty")
	}
	if int64(len(data)) > lim.bytes() {
		return nil, fmt.Errorf("exechead: the file is %d bytes, over the cap of %d", len(data), lim.bytes())
	}
	if data[len(data)-1] != '\n' {
		return nil, errors.New("exechead: the file does not end in a newline")
	}
	sum := sha256.Sum256(data)
	f := &File{Bytes: int64(len(data)), SHA256: hex.EncodeToString(sum[:])}
	rest := data
	for n := 0; len(rest) > 0; n++ {
		i := bytes.IndexByte(rest, '\n')
		line := rest[:i]
		rest = rest[i+1:]
		if len(line) == 0 {
			return nil, fmt.Errorf("exechead: line %d is empty", n+1)
		}
		if len(line) > MaxLineBytes {
			return nil, fmt.Errorf("exechead: line %d is %d bytes, over the cap of %d", n+1, len(line), MaxLineBytes)
		}
		if n == 0 {
			h, err := decodeHeader(line)
			if err != nil {
				return nil, fmt.Errorf("exechead: line 1 (the header): %w", err)
			}
			if err := want.check(h); err != nil {
				return nil, fmt.Errorf("exechead: line 1 (the header): %w", err)
			}
			f.Header = h
			continue
		}
		if n > lim.lines() {
			return nil, fmt.Errorf("exechead: more than %d notification lines", lim.lines())
		}
		l, err := decodeLine(line, n-1)
		if err != nil {
			return nil, fmt.Errorf("exechead: line %d: %w", n+1, err)
		}
		f.Lines = append(f.Lines, l)
	}
	return f, nil
}

func (e *Expect) check(h Header) error {
	if e == nil {
		return nil
	}
	if e.Nonce != "" && h.Nonce != e.Nonce {
		return errors.New("the nonce is not the step sidecar's")
	}
	if e.ActionSHA256 != "" && h.ActionSHA256 != e.ActionSHA256 {
		return errors.New("the file belongs to another action")
	}
	if e.CheckStep && h.Step != e.Step {
		return fmt.Errorf("the file belongs to step %d, not %d", h.Step, e.Step)
	}
	return nil
}

func decodeHeader(line []byte) (Header, error) {
	var w wireHeader
	if err := legacy.PolicyDecode(line, &w); err != nil {
		return Header{}, err
	}
	if w.V == nil || w.Kernel == nil || w.Nonce == nil || w.ActionSHA256 == nil || w.Step == nil {
		return Header{}, errors.New("a key is missing")
	}
	h := Header{V: *w.V, Kernel: *w.Kernel, Nonce: *w.Nonce, ActionSHA256: *w.ActionSHA256, Step: *w.Step}
	if err := checkHeader(h); err != nil {
		return Header{}, err
	}
	again, err := encodeCanonical(wireHeader{V: &h.V, Kernel: &h.Kernel, Nonce: &h.Nonce, ActionSHA256: &h.ActionSHA256, Step: &h.Step})
	if err != nil || !bytes.Equal(again, line) {
		return Header{}, errors.New("not the canonical encoding (spacing, key order or escapes differ)")
	}
	return h, nil
}

func decodeLine(line []byte, seq int) (Line, error) {
	var w wireLine
	if err := legacy.PolicyDecode(line, &w); err != nil {
		return Line{}, err
	}
	if w.Seq == nil || w.Tid == nil || w.Tgid == nil || w.Syscall == nil || w.Path == nil || w.Status == nil {
		return Line{}, errors.New("a key is missing")
	}
	l := Line{Seq: *w.Seq, Tid: *w.Tid, Tgid: *w.Tgid, Syscall: *w.Syscall, Path: *w.Path, Status: Status(*w.Status)}
	if l.Seq != seq {
		return Line{}, fmt.Errorf("seq is %d, want %d (a gap, a repeat or a reorder)", l.Seq, seq)
	}
	if !l.Status.Valid() {
		return Line{}, fmt.Errorf("status %q is not one of the eleven", clip(*w.Status))
	}
	extras := 0
	for _, present := range []bool{w.Dev != nil, w.Ino != nil, w.Head != nil, w.SHA256 != nil} {
		if present {
			extras++
		}
	}
	switch {
	case l.Status == StatusConfirmed && extras != 4:
		return Line{}, errors.New("a confirmed line needs dev, ino, head and sha256")
	case l.Status != StatusConfirmed && extras != 0:
		return Line{}, fmt.Errorf("a %s line may not carry dev, ino, head or sha256", l.Status)
	}
	if l.Status == StatusConfirmed {
		l.Dev, l.Ino, l.SHA256 = *w.Dev, *w.Ino, *w.SHA256
		if base64.StdEncoding.DecodedLen(len(*w.Head)) > HeadLimit+2 {
			return Line{}, errors.New("the head is over the limit")
		}
		head, err := base64.StdEncoding.Strict().DecodeString(*w.Head)
		if err != nil {
			return Line{}, fmt.Errorf("head: %v", err)
		}
		if len(head) > HeadLimit {
			return Line{}, fmt.Errorf("a head of %d bytes (the most is %d)", len(head), HeadLimit)
		}
		l.Head = head
		if l.Head == nil {
			l.Head = []byte{}
		}
	}
	if err := checkLine(l); err != nil {
		return Line{}, err
	}
	again, err := EncodeLine(l)
	if err != nil {
		return Line{}, err
	}
	if !bytes.Equal(again, line) {
		return Line{}, errors.New("not the canonical encoding (spacing, key order or escapes differ)")
	}
	return l, nil
}
