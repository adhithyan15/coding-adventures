// Manifests (KILN04 section 8) are a STORE-ONLY record kind that lives beside
// the pass ledger.  A manifest says "while tracing target T under weak key W,
// the BUILD read exactly these names".  It carries names, never digests, never
// file content, never an environment value.
//
// Why this is a sub-area of package ledger and not its own package: manifests
// reuse the ledger's unexported machinery verbatim (segment-name pattern,
// listSegments, openRead, requireRegular, writeAtomic, randHexString,
// checkOutDir, the Authority gate, checkText).  A separate package would have
// to export all of that, widening the ledger's surface for one caller.  The
// isolation that matters is on disk (a "manifests/" subdirectory with its own
// record format), and that is tested explicitly.
//
// TRUST.  A manifest can never cause a hit by itself: the strong key needs a
// PASS record for exactly those entries' current digests, and only a trusted
// run writes passes.  A forged or stale manifest can therefore only cost time.
// That is why manifests are not restamped and why the reader may DROP a record
// it cannot accept, where the pass ledger must fail whole.
package ledger

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"strings"
	"time"
	"unicode/utf8"

	kkey "github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// Manifest caps (KILN04 section 8).
const (
	// MaxManifestEntries caps entries in one record.
	MaxManifestEntries = 8192
	// MaxManifestNameBytes caps one entry name, prefix included.
	MaxManifestNameBytes = 512
	// MaxManifestLineBytes caps one encoded line, newline included.
	MaxManifestLineBytes = 1 << 20
	// MaxManifestSegmentBytes caps one segment file.
	MaxManifestSegmentBytes = 64 << 20
	// MaxManifestStoreBytes caps the total segment bytes of one store.
	MaxManifestStoreBytes = 256 << 20
	// ManifestKeep is K: distinct ids retained per weak key.
	ManifestKeep = 4
	// ManifestMaxAge is how long a manifest is retained.
	ManifestMaxAge = 30 * 24 * time.Hour
	// ManifestsDir is the subdirectory of a ledger directory holding them.
	ManifestsDir = "manifests"
	// KindManifest is the constant "kind" of a manifest line.  It is a
	// store-only kind: it is not a Record kind and the pass reader refuses it.
	KindManifest = "manifest"
)

// ErrManifestTooLarge is returned (wrapped) when a record to be emitted
// exceeds a per-record cap.  The caller reports `manifest-too-large` for that
// action (KILN04 section 8).
var ErrManifestTooLarge = errors.New("manifest-too-large")

// Manifest is one traced read-set.  The id is not a field: it is always
// computed from Entries (ID), so a stored id can never disagree with its
// entries without the reader noticing.
type Manifest struct {
	Weak     [32]byte
	Target   string   // the package rel_path (KILN02 section 3.5 rules)
	OS       string   //
	Arch     string   //
	Entries  []string // sorted, unique, "pkg:" / "path:" / "list:" names
	Revision string
	Run      string
	At       time.Time // UTC, whole seconds
}

// manifestWire is the JSON shape.  Field order here is the canonical order.
type manifestWire struct {
	Kind     string   `json:"kind"`
	ID       string   `json:"id"`
	Weak     string   `json:"weak"`
	Target   string   `json:"target"`
	OS       string   `json:"os"`
	Arch     string   `json:"arch"`
	Entries  []string `json:"entries"`
	Revision string   `json:"revision"`
	Run      string   `json:"run"`
	At       string   `json:"at"`
}

// ID is the SHA-256 of the canonical entries list: the compact JSON array of
// the names (no HTML escaping), exactly the bytes that appear after
// `"entries":` in the line.  A nil list is the empty array.
func (m Manifest) ID() [32]byte {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(entriesOrEmpty(m.Entries)) // a []string cannot fail
	return sha256.Sum256(bytes.TrimSuffix(buf.Bytes(), []byte("\n")))
}

// WeakHex returns the weak key as 64 lowercase hex characters.
func (m Manifest) WeakHex() string { return hex.EncodeToString(m.Weak[:]) }

func entriesOrEmpty(e []string) []string {
	if e == nil {
		return []string{}
	}
	return e
}

// checkEntry validates one entry name's STRUCTURE (allowlisted prefix, text,
// path rules).  Its length is a cap, checked separately so an over-long but
// well-formed name is dropped by a reader rather than treated as corruption.
func checkEntry(e string) error {
	if err := checkText("entry", e, math.MaxInt, true); err != nil {
		return err
	}
	prefix, rest, ok := strings.Cut(e, ":")
	if !ok {
		return fmt.Errorf("entry %q has no prefix", short40(e))
	}
	switch prefix {
	case "pkg", "path":
		if err := kkey.ValidatePath(rest); err != nil {
			return fmt.Errorf("entry %q: %w", short40(e), err)
		}
	case "list":
		if rest == "." {
			return nil // the repository root
		}
		if err := kkey.ValidatePath(rest); err != nil {
			return fmt.Errorf("entry %q: %w", short40(e), err)
		}
	default:
		return fmt.Errorf("entry %q: prefix %q is not allowed (want pkg:, path: or list:)", short40(e), prefix)
	}
	return nil
}

func short40(s string) string {
	if len(s) <= 40 {
		return s
	}
	i := 40
	for i > 0 && !utf8.RuneStart(s[i]) {
		i--
	}
	return s[:i] + "..."
}

// validateStructure refuses a malformed manifest.  It checks everything that,
// if wrong, means the writer was broken or hostile (reader: whole store
// unusable).  It does NOT check the per-record caps; see checkCaps.
func (m Manifest) validateStructure() error {
	if err := checkText("target", m.Target, math.MaxInt, true); err != nil {
		return err
	}
	if err := kkey.ValidatePath(m.Target); err != nil {
		return fmt.Errorf("target: %w", err)
	}
	if len(m.Target) > MaxTargetBytes {
		return fmt.Errorf("target is %d bytes, limit %d", len(m.Target), MaxTargetBytes)
	}
	for _, f := range []struct {
		name, v  string
		required bool
	}{{"os", m.OS, true}, {"arch", m.Arch, true}, {"revision", m.Revision, false}, {"run", m.Run, false}} {
		if err := checkText(f.name, f.v, MaxFieldBytes, f.required); err != nil {
			return err
		}
	}
	if m.At.IsZero() || m.At.Location() != time.UTC || m.At.Nanosecond() != 0 {
		return errors.New("at must be a non-zero UTC time in whole seconds")
	}
	if y := m.At.Year(); y > 9999 || y < 0 {
		return errors.New("at year out of range")
	}
	for i, e := range m.Entries {
		if err := checkEntry(e); err != nil {
			return err
		}
		if i > 0 && m.Entries[i-1] >= e {
			if m.Entries[i-1] == e {
				return fmt.Errorf("duplicate entry %q", short40(e))
			}
			return fmt.Errorf("entries are not sorted at %q", short40(e))
		}
	}
	return nil
}

// checkCaps enforces the per-record caps on entry count and name length.
// The line-length cap is checked on the encoded bytes.
func (m Manifest) checkCaps() error {
	if len(m.Entries) > MaxManifestEntries {
		return fmt.Errorf("%w: %d entries, limit %d", ErrManifestTooLarge, len(m.Entries), MaxManifestEntries)
	}
	for _, e := range m.Entries {
		if len(e) > MaxManifestNameBytes {
			return fmt.Errorf("%w: entry of %d bytes, limit %d", ErrManifestTooLarge, len(e), MaxManifestNameBytes)
		}
	}
	return nil
}

// Validate refuses a malformed or over-cap manifest; it never repairs one.
// A cap violation wraps ErrManifestTooLarge.
func (m Manifest) Validate() error {
	if err := m.validateStructure(); err != nil {
		return err
	}
	return m.checkCaps()
}

// encode renders the canonical line (newline included) with no validation.
func (m Manifest) encode() []byte {
	id := m.ID()
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(manifestWire{KindManifest, hex.EncodeToString(id[:]), m.WeakHex(), m.Target,
		m.OS, m.Arch, entriesOrEmpty(m.Entries), m.Revision, m.Run, m.At.Format(time.RFC3339)})
	return buf.Bytes()
}

// Encode validates m and returns its canonical JSON line, newline included.
// A record over any per-record cap (entries, name bytes, line bytes) is an
// error wrapping ErrManifestTooLarge: the writer must not emit it.
func (m Manifest) Encode() ([]byte, error) {
	if err := m.Validate(); err != nil {
		return nil, err
	}
	line := m.encode()
	if len(line) > MaxManifestLineBytes {
		return nil, fmt.Errorf("%w: line is %d bytes, limit %d", ErrManifestTooLarge, len(line), MaxManifestLineBytes)
	}
	return line, nil
}

// errOverCap marks a record that is structurally sound but over a per-record
// cap.  Readers DROP such a record (and report it) rather than failing.
var errOverCap = errors.New("over a per-record cap")

// decodeManifestLine parses one line (without its newline) strictly.  Error
// classes, which the reader's policy depends on:
//
//   - an error wrapping errOverCap: well-formed and canonical but over a cap
//     (entry count, name length): the reader drops this record;
//   - any other error: STRUCTURAL corruption (bad JSON, unknown field,
//     non-canonical spelling, bad entry, id mismatch): the reader refuses the
//     whole store.
//
// Structure is judged first, so a line that is both over-cap and
// non-canonical is corruption, never a quiet drop.  A line longer than
// MaxManifestLineBytes is not parsed at all; the segment reader drops it.
func decodeManifestLine(line []byte) (Manifest, error) {
	if !utf8.Valid(line) {
		return Manifest{}, errors.New("line is not valid UTF-8")
	}
	dec := json.NewDecoder(bytes.NewReader(line))
	dec.DisallowUnknownFields()
	var w manifestWire
	if err := dec.Decode(&w); err != nil {
		return Manifest{}, fmt.Errorf("bad JSON: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return Manifest{}, errors.New("trailing data after the JSON value")
	}
	if w.Kind != KindManifest {
		return Manifest{}, fmt.Errorf("kind is %q, want %q", w.Kind, KindManifest)
	}
	id, err := decodeHex32("id", w.ID)
	if err != nil {
		return Manifest{}, err
	}
	weak, err := decodeHex32("weak", w.Weak)
	if err != nil {
		return Manifest{}, err
	}
	at, err := time.Parse(time.RFC3339, w.At)
	if err != nil {
		return Manifest{}, fmt.Errorf("bad at: %w", err)
	}
	m := Manifest{Weak: weak, Target: w.Target, OS: w.OS, Arch: w.Arch, Entries: w.Entries,
		Revision: w.Revision, Run: w.Run, At: at.UTC()}
	if err := m.validateStructure(); err != nil {
		return Manifest{}, err
	}
	// The id is checked before the canonical comparison so a forged id gets
	// its own message (the canonical encoding embeds the COMPUTED id).
	if id != m.ID() {
		return Manifest{}, errors.New("id does not match the entries")
	}
	if !bytes.Equal(m.encode(), append(append([]byte{}, line...), '\n')) {
		return Manifest{}, errors.New("line is not in canonical form")
	}
	if err := m.checkCaps(); err != nil {
		return Manifest{}, fmt.Errorf("%w: %v", errOverCap, err)
	}
	return m, nil
}

func decodeHex32(name, s string) ([32]byte, error) {
	var out [32]byte
	if len(s) != 64 {
		return out, fmt.Errorf("%s is %d characters, want 64", name, len(s))
	}
	for i := 0; i < 64; i++ {
		if c := s[i]; (c < '0' || c > '9') && (c < 'a' || c > 'f') {
			return out, fmt.Errorf("%s must be lowercase hex", name)
		}
	}
	_, _ = hex.Decode(out[:], []byte(s))
	return out, nil
}

// runLess orders run identifiers.  CI run ids are decimal and grow, so two
// numeric runs compare as numbers (without leading zeros, by length then
// bytes, so there is no overflow); numeric runs sort before non-numeric ones;
// non-numeric runs compare as bytes.  It is a strict total order.
func runLess(a, b string) bool {
	an, bn := isDigits(a), isDigits(b)
	switch {
	case an && bn:
		ta, tb := strings.TrimLeft(a, "0"), strings.TrimLeft(b, "0")
		if len(ta) != len(tb) {
			return len(ta) < len(tb)
		}
		if ta != tb {
			return ta < tb
		}
		return a < b // same number, different zero padding
	case an != bn:
		return an
	}
	return a < b
}

func isDigits(s string) bool {
	if s == "" {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return false
		}
	}
	return true
}

// mEntry is a manifest with its position, which breaks ordering ties.
type mEntry struct {
	m    Manifest
	id   [32]byte // m.ID(), computed once when the retainer accepts the record
	seq  uint64
	name string
	line int
}

// newer reports whether a supersedes b.  THE ORDERING RULE (KILN04 section 8):
// by run, then by at; remaining ties go, as in the pass ledger, to the higher
// segment sequence, the lexically higher segment name, then the later line.
// Nothing else (file modification time, directory order) is consulted.
func (a mEntry) newer(b mEntry) bool {
	switch {
	case a.m.Run != b.m.Run:
		return runLess(b.m.Run, a.m.Run)
	case !a.m.At.Equal(b.m.At):
		return a.m.At.After(b.m.At)
	case a.seq != b.seq:
		return a.seq > b.seq
	case a.name != b.name:
		return a.name > b.name
	}
	return a.line > b.line
}
