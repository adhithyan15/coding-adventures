// Package ledger is Kiln's directory ledger (KILN00 section 10): a set of
// immutable segment files, each holding JSON-lines pass and tombstone records.
//
// The ledger stores hashes only.  A record says "the action with this key
// passed (or was revoked)", plus enough provenance to explain why a target was
// skipped.  A missed record only costs time; a FALSE record is the real danger
// (a failing target skipped forever).  Every choice here therefore favours
// refusing: validation never repairs, an unreadable segment fails the whole
// lookup rather than being skipped, and writing requires the host's trust.
package ledger

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"time"
	"unicode"
	"unicode/utf8"
)

// Limits.  They bound memory for the most trusted step in the system
// (ledger-merge), which parses files produced by other jobs.
const (
	// MaxTargetBytes caps the target label ("rust/parser:test").
	MaxTargetBytes = 512
	// MaxFieldBytes caps os, arch, kiln, revision and run.
	MaxFieldBytes = 128
	// MaxLineBytes caps one encoded record line, newline included.  Eight
	// fields at their caps plus JSON syntax fit comfortably.
	MaxLineBytes = 4096
	// MaxRecordsPerSegment caps records in one file.
	MaxRecordsPerSegment = 50_000
	// MaxSegmentBytes caps the size of one file.
	MaxSegmentBytes = 16 << 20
	// MaxSegments caps the segment files read from one directory.
	MaxSegments = 10_000
	// MaxDirRecords caps records held in memory across one directory.
	MaxDirRecords = 1_000_000
)

// Kinds of record.
const (
	// KindPass: the keyed action passed on a trusted run.
	KindPass = "pass"
	// KindTombstone: the daily uncached run disagreed; stop honouring passes
	// for this key (KILN00 section 10.5).
	KindTombstone = "tombstone"
)

// Record is one ledger entry.  Key is the 32-byte action key (KILN02).
type Record struct {
	Key      [32]byte
	Target   string
	OS       string
	Arch     string
	Kiln     string // schema/semantics version of the Kiln that wrote it
	Revision string
	Run      string
	At       time.Time // UTC, whole seconds
	Kind     string
}

// wire is the JSON shape.  Field order here is the canonical order.
type wire struct {
	Key      string `json:"key"`
	Target   string `json:"target"`
	OS       string `json:"os"`
	Arch     string `json:"arch"`
	Kiln     string `json:"kiln"`
	Revision string `json:"revision"`
	Run      string `json:"run"`
	At       string `json:"at"`
	Kind     string `json:"kind"`
}

// checkText accepts valid UTF-8 without control characters (C0, DEL, C1) or
// invisible format characters (category Cf: zero-width and bidirectional
// overrides, which can make a printed label read differently from its bytes),
// within max bytes.  Required fields must also be non-empty.
func checkText(name, s string, max int, required bool) error {
	switch {
	case required && s == "":
		return fmt.Errorf("%s is empty", name)
	case len(s) > max:
		return fmt.Errorf("%s is %d bytes, limit %d", name, len(s), max)
	case !utf8.ValidString(s):
		return fmt.Errorf("%s is not valid UTF-8", name)
	}
	for _, r := range s {
		if unicode.IsControl(r) || unicode.Is(unicode.Cf, r) {
			return fmt.Errorf("%s contains control or format character U+%04X", name, r)
		}
	}
	return nil
}

// Validate refuses a malformed record; it never repairs one.
func (r Record) Validate() error {
	if err := checkText("target", r.Target, MaxTargetBytes, true); err != nil {
		return err
	}
	for _, f := range []struct {
		name, v  string
		required bool
	}{{"os", r.OS, true}, {"arch", r.Arch, true}, {"kiln", r.Kiln, true},
		{"revision", r.Revision, false}, {"run", r.Run, false}} {
		if err := checkText(f.name, f.v, MaxFieldBytes, f.required); err != nil {
			return err
		}
	}
	if r.Kind != KindPass && r.Kind != KindTombstone {
		return fmt.Errorf("unknown kind %q", r.Kind)
	}
	if r.At.IsZero() || r.At.Location() != time.UTC || r.At.Nanosecond() != 0 {
		return errors.New("at must be a non-zero UTC time in whole seconds")
	}
	if y := r.At.Year(); y > 9999 || y < 0 {
		return errors.New("at year out of range")
	}
	return nil
}

// Hex returns the key as 64 lowercase hex characters.
func (r Record) Hex() string { return hex.EncodeToString(r.Key[:]) }

// Encode validates r and returns its canonical JSON line, newline included.
// No line-length check is needed here: the field caps bound the worst case (a
// field full of U+2028, which JSON writes as six bytes) to about 2.3 KB, well
// under MaxLineBytes.  DecodeLine still enforces MaxLineBytes on what it reads.
func (r Record) Encode() ([]byte, error) {
	if err := r.Validate(); err != nil {
		return nil, err
	}
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	// Encoding a struct of strings cannot fail.
	_ = enc.Encode(wire{r.Hex(), r.Target, r.OS, r.Arch, r.Kiln, r.Revision,
		r.Run, r.At.Format(time.RFC3339), r.Kind})
	return buf.Bytes(), nil
}

// DecodeLine parses one line (without its newline) strictly.  After the JSON
// parses (unknown fields and trailing data refused) the record is validated
// and re-encoded; the line must equal that canonical encoding byte for byte.
// That one comparison rejects duplicate keys, reordered fields, extra
// whitespace, escapes, fractional seconds and non-UTC offsets, so every
// accepted line has exactly one spelling.
func DecodeLine(line []byte) (Record, error) {
	if len(line)+1 > MaxLineBytes {
		return Record{}, fmt.Errorf("line is %d bytes, limit %d", len(line), MaxLineBytes-1)
	}
	if !utf8.Valid(line) {
		return Record{}, errors.New("line is not valid UTF-8")
	}
	dec := json.NewDecoder(bytes.NewReader(line))
	dec.DisallowUnknownFields()
	var w wire
	if err := dec.Decode(&w); err != nil {
		return Record{}, fmt.Errorf("bad JSON: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return Record{}, errors.New("trailing data after the JSON value")
	}
	if len(w.Key) != 64 {
		return Record{}, fmt.Errorf("key is %d characters, want 64", len(w.Key))
	}
	for i := 0; i < 64; i++ {
		if c := w.Key[i]; (c < '0' || c > '9') && (c < 'a' || c > 'f') {
			return Record{}, errors.New("key must be lowercase hex")
		}
	}
	at, err := time.Parse(time.RFC3339, w.At)
	if err != nil {
		return Record{}, fmt.Errorf("bad at: %w", err)
	}
	rec := Record{Target: w.Target, OS: w.OS, Arch: w.Arch, Kiln: w.Kiln,
		Revision: w.Revision, Run: w.Run, At: at.UTC(), Kind: w.Kind}
	_, _ = hex.Decode(rec.Key[:], []byte(w.Key)) // validated above
	canon, err := rec.Encode()
	if err != nil {
		return Record{}, err
	}
	if !bytes.Equal(canon[:len(canon)-1], line) {
		return Record{}, errors.New("line is not in canonical form")
	}
	return rec, nil
}

// ParseSegment reads a whole segment: JSON lines, each ending in "\n", no
// blank lines, at most MaxRecordsPerSegment records and MaxSegmentBytes bytes.
// It never reads more than MaxSegmentBytes+1 bytes from r.
func ParseSegment(r io.Reader) ([]Record, error) {
	return parseSegment(r, MaxSegmentBytes, MaxRecordsPerSegment)
}

// parseSegment is ParseSegment with injectable limits, so tests need not build
// 16 MiB files.  We ask the LimitReader for limit+1 bytes so "exactly limit" is
// accepted and "one more" is detectable rather than silently truncated.
func parseSegment(r io.Reader, maxBytes int64, maxRecords int) ([]Record, error) {
	data, err := io.ReadAll(io.LimitReader(r, maxBytes+1))
	if err != nil {
		return nil, err
	}
	if int64(len(data)) > maxBytes {
		return nil, fmt.Errorf("segment larger than %d bytes", maxBytes)
	}
	var out []Record
	for n := 1; len(data) > 0; n++ {
		i := bytes.IndexByte(data, '\n')
		if i < 0 {
			return nil, fmt.Errorf("line %d: missing final newline", n)
		}
		if len(out) >= maxRecords {
			return nil, fmt.Errorf("more than %d records", maxRecords)
		}
		rec, err := DecodeLine(data[:i])
		if err != nil {
			return nil, fmt.Errorf("line %d: %w", n, err)
		}
		out = append(out, rec)
		data = data[i+1:]
	}
	return out, nil
}

// EncodeSegment encodes records, in order, as one segment's bytes, enforcing
// the same caps ParseSegment enforces.
func EncodeSegment(records []Record) ([]byte, error) {
	if len(records) > MaxRecordsPerSegment {
		return nil, fmt.Errorf("%d records exceed the segment limit %d", len(records), MaxRecordsPerSegment)
	}
	var out []byte
	for i, r := range records {
		line, err := r.Encode()
		if err != nil {
			return nil, fmt.Errorf("record %d: %w", i+1, err)
		}
		out = append(out, line...)
		if len(out) > MaxSegmentBytes {
			return nil, fmt.Errorf("segment exceeds %d bytes", MaxSegmentBytes)
		}
	}
	return out, nil
}
