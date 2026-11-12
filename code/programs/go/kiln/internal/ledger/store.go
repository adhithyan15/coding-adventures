package ledger

import (
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strconv"
)

// ErrUntrusted is returned by Store.Record when the host did not vouch for the
// run (KILN00 section 10.2): only trusted runs may write to the shared ledger.
var ErrUntrusted = errors.New("ledger: refusing to record from an untrusted run")

// Segment file names look like "0000000042-0123456789abcdef.jsonl": a
// ten-digit decimal sequence, a dash, sixteen lowercase hex characters of
// randomness, and ".jsonl".  The sequence orders segments; the randomness lets
// concurrent writers that picked the same sequence coexist.  The pattern is
// checked before any path is built, so a name can never contain a separator or
// "..", and anything that does not match is never opened.
const (
	seqDigits  = 10
	randHex    = 16
	segSuffix  = ".jsonl"
	segNameLen = seqDigits + 1 + randHex + len(segSuffix)
	maxSeq     = 9_999_999_999
)

// segmentName builds a name; seq must be <= maxSeq and rnd 16 hex characters.
func segmentName(seq uint64, rnd string) string {
	return fmt.Sprintf("%0*d-%s%s", seqDigits, seq, rnd, segSuffix)
}

// parseSegmentName reports whether name is a valid segment name and its
// sequence number.
func parseSegmentName(name string) (uint64, bool) {
	if len(name) != segNameLen || name[seqDigits] != '-' || name[segNameLen-len(segSuffix):] != segSuffix {
		return 0, false
	}
	for i := 0; i < seqDigits; i++ {
		if name[i] < '0' || name[i] > '9' {
			return 0, false
		}
	}
	for _, c := range []byte(name[seqDigits+1 : seqDigits+1+randHex]) {
		if (c < '0' || c > '9') && (c < 'a' || c > 'f') {
			return 0, false
		}
	}
	seq, _ := strconv.ParseUint(name[:seqDigits], 10, 64)
	return seq, true
}

// Store is a ledger directory.  Segments are immutable: a Store only ever adds
// files, it never rewrites or deletes one.
type Store struct {
	dir string
	fs  fsOps
}

// Open returns a Store over dir.  The directory need not exist yet; a missing
// directory is an empty ledger.
func Open(dir string) *Store { return &Store{dir: dir, fs: realFS} }

// segInfo names one valid segment file.
type segInfo struct {
	name string
	seq  uint64
}

// listSegments lists the valid segment files of dir, in segment order
// (sequence, then name).  Entries whose names do not match the pattern (temp
// files of an interrupted write, a README) are ignored: they are never read.
// An entry that DOES match but is a symlink, directory or other non-regular
// file is an error.  os.ReadDir reports each entry's own type without
// following symlinks, so a symlinked segment is seen as a symlink.
func listSegments(dir string) ([]segInfo, error) {
	entries, err := os.ReadDir(dir)
	if errors.Is(err, fs.ErrNotExist) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	var out []segInfo
	for _, e := range entries {
		seq, ok := parseSegmentName(e.Name())
		if !ok {
			continue
		}
		if !e.Type().IsRegular() {
			return nil, fmt.Errorf("ledger: segment %s is not a regular file", e.Name())
		}
		if len(out) >= MaxSegments {
			return nil, fmt.Errorf("ledger: more than %d segments", MaxSegments)
		}
		out = append(out, segInfo{e.Name(), seq})
	}
	// ReadDir sorts by name, and the sequence is zero-padded, so this is
	// already segment order; sort anyway so correctness does not rest on it.
	sort.Slice(out, func(i, j int) bool {
		if out[i].seq != out[j].seq {
			return out[i].seq < out[j].seq
		}
		return out[i].name < out[j].name
	})
	return out, nil
}

// readRecordFile opens path, checks on the open descriptor that it is a
// regular file (a separate stat of the path would leave a TOCTOU window), and
// parses it with the segment limits.
func readRecordFile(path string) ([]Record, error) {
	f, err := openRead(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !info.Mode().IsRegular() {
		return nil, fmt.Errorf("ledger: %s is not a regular file", filepath.Base(path))
	}
	recs, err := ParseSegment(f)
	if err != nil {
		return nil, fmt.Errorf("ledger: %s: %w", filepath.Base(path), err)
	}
	return recs, nil
}

// entry is a record with its position, which decides which of two records for
// one key is newer.
type entry struct {
	rec  Record
	seq  uint64 // segment sequence
	name string // segment file name
	line int    // 1-based line within the segment
}

// newer reports whether a supersedes b.  THE ORDERING RULE: compare the
// records' At times first; ties go to the higher segment sequence, then to the
// lexically higher segment name, then to the later line.  Nothing else (file
// modification time, directory listing order) is consulted, so the result is
// the same on every machine and every filesystem.
func (a entry) newer(b entry) bool {
	switch {
	case !a.rec.At.Equal(b.rec.At):
		return a.rec.At.After(b.rec.At)
	case a.seq != b.seq:
		return a.seq > b.seq
	case a.name != b.name:
		return a.name > b.name
	}
	return a.line > b.line
}

// loadDir reads every segment of dir strictly.  ANY unreadable or invalid
// segment fails the whole load.  Skipping a bad segment would be unsafe: it
// might hold the tombstone that revokes a pass, and ignoring it would resurrect
// that pass as a false hit.  Failing only costs a rebuild.
func loadDir(dir string) ([]entry, error) {
	segs, err := listSegments(dir)
	if err != nil {
		return nil, err
	}
	var all []entry
	for _, s := range segs {
		recs, err := readRecordFile(filepath.Join(dir, s.name))
		if err != nil {
			return nil, err
		}
		if len(all)+len(recs) > MaxDirRecords {
			return nil, fmt.Errorf("ledger: more than %d records in %s", MaxDirRecords, dir)
		}
		for i, r := range recs {
			all = append(all, entry{r, s.seq, s.name, i + 1})
		}
	}
	return all, nil
}

// newest returns the newest entry per key.
func newest(all []entry) map[[32]byte]entry {
	m := make(map[[32]byte]entry, len(all))
	for _, e := range all {
		if cur, ok := m[e.rec.Key]; !ok || e.newer(cur) {
			m[e.rec.Key] = e
		}
	}
	return m
}

// Lookup returns, for each requested key, its newest record if that record is
// a pass.  A key whose newest record is a tombstone, or that has no record, is
// absent from the result (a miss).  A newer pass after a tombstone un-shadows
// the key.  On any error the result is nil: see loadDir for why.
func (s *Store) Lookup(keys [][32]byte) (map[[32]byte]Record, error) {
	all, err := loadDir(s.dir)
	if err != nil {
		return nil, err
	}
	best := newest(all)
	out := make(map[[32]byte]Record)
	for _, k := range keys {
		if e, ok := best[k]; ok && e.rec.Kind == KindPass {
			out[k] = e.rec
		}
	}
	return out, nil
}

// Record appends records as one NEW segment.  trusted is the Host's verdict
// (KILN00 section 10.2); when false nothing is written and ErrUntrusted is
// returned.  Every record is validated first and the batch is refused whole if
// any is invalid or the batch exceeds the segment caps.  An empty batch writes
// nothing.
func (s *Store) Record(records []Record, trusted bool) error {
	if !trusted {
		return ErrUntrusted
	}
	if len(records) == 0 {
		return nil
	}
	data, err := EncodeSegment(records)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(s.dir, 0o755); err != nil {
		return err
	}
	segs, err := listSegments(s.dir)
	if err != nil {
		return err
	}
	var next uint64 = 1
	if n := len(segs); n > 0 {
		next = segs[n-1].seq + 1
	}
	if next > maxSeq {
		return errors.New("ledger: segment sequence exhausted")
	}
	var rnd [randHex / 2]byte
	if _, err := rand.Read(rnd[:]); err != nil {
		return err
	}
	return writeAtomic(s.fs, s.dir, segmentName(next, hex.EncodeToString(rnd[:])), data)
}

// tmpFile is the part of *os.File that writeAtomic uses.
type tmpFile interface {
	io.Writer
	Sync() error
	Close() error
	Name() string
}

// fsOps is the seam tests use to inject failures into writeAtomic.
type fsOps struct {
	create func(dir string) (tmpFile, error)
	rename func(oldpath, newpath string) error
}

var realFS = fsOps{
	create: func(dir string) (tmpFile, error) { return os.CreateTemp(dir, ".tmp-*") },
	rename: os.Rename,
}

// writeAtomic makes name appear in dir complete or not at all: write to a temp
// file in the SAME directory (so the rename stays on one filesystem), fsync it,
// rename it into place, then best-effort fsync the directory so the rename
// survives a crash.  A reader therefore never sees a partial segment; a crash
// leaves at most a ".tmp-*" file, which listSegments ignores.  On any failure
// the temp file is removed.
func writeAtomic(o fsOps, dir, name string, data []byte) (err error) {
	f, err := o.create(dir)
	if err != nil {
		return err
	}
	tmp := f.Name()
	defer func() {
		if err != nil {
			_ = os.Remove(tmp)
		}
	}()
	if _, err = f.Write(data); err != nil {
		_ = f.Close()
		return err
	}
	if err = f.Sync(); err != nil {
		_ = f.Close()
		return err
	}
	if err = f.Close(); err != nil {
		return err
	}
	if err = o.rename(tmp, filepath.Join(dir, name)); err != nil {
		return err
	}
	syncDir(dir)
	return nil
}

// syncDir flushes directory metadata where the OS allows it (Windows does not
// permit syncing a directory handle; the error is ignored on purpose, since
// the rename has already happened and a lost rename only loses a record).
func syncDir(dir string) {
	if d, err := os.Open(dir); err == nil {
		_ = d.Sync()
		_ = d.Close()
	}
}
