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
	"slices"
	"sort"
	"strconv"
	"time"
)

// ErrUntrusted is returned when the Authority does not permit a write
// (KILN00 section 10.2): only trusted runs may write to the shared ledger.
var ErrUntrusted = errors.New("ledger: refusing to write: the run is not authorised")

// MaxClockSkew is how far after "now" a snapshot or delta record's At may be.  Ordering trusts
// At, so a far-future At would outrank every tombstone; anything beyond this
// slack is refused.
const MaxClockSkew = 5 * time.Minute

// MaxRecordSkew is the tighter slack for Store.Record, where the record's
// writer and the clock are the same machine.
const MaxRecordSkew = time.Minute

// MaxDirBytes caps the total bytes read from one ledger directory.
const MaxDirBytes = 256 << 20

// Scope says which kind of ledger a Store is.
type Scope int

const (
	// Shared is the CI ledger; writing needs Authority.Trusted().
	Shared Scope = iota + 1
	// Local is a developer's own <repo>/.kiln/ledger; writing needs
	// Authority.WritesLocalLedger().
	Local
)

// Authority is the Host's verdict about the current run (KILN00 section 13).
// It is an interface, not a bool, so a stray `true` cannot grant write access
// and the shared and local ledgers cannot be confused.  The Host adapters
// implement exactly these two methods: Local is (false, true); a trusted
// GitHub run is (true, false); an untrusted run is (false, false).
type Authority interface {
	Trusted() bool
	WritesLocalLedger() bool
}

// authorize refuses a nil Authority or one that does not permit scope.
func authorize(a Authority, scope Scope) error {
	if a == nil {
		return ErrUntrusted
	}
	switch {
	case scope == Shared && a.Trusted():
		return nil
	case scope == Local && a.WritesLocalLedger():
		return nil
	}
	return ErrUntrusted
}

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
	dir   string
	scope Scope
	fs    fsOps
}

// Open returns a Store over dir.  The directory need not exist yet; a missing
// directory is an empty ledger.
func Open(dir string, scope Scope) *Store { return &Store{dir: dir, scope: scope, fs: realFS} }

// segInfo names one valid segment file.
type segInfo struct {
	name string
	seq  uint64
}

// listSegments lists the valid segment files of dir, in segment order
// (sequence, then name).  Entries whose names do not match the pattern (temp
// files of an interrupted write, a README) are ignored: they are never read.
// An entry that DOES match but is a symlink, directory or other non-regular
// file is an error.  ReadDir reports each entry's own type without following
// symlinks, so a symlinked segment is seen as a symlink.
func listSegments(dir string) ([]segInfo, error) {
	d, err := os.Open(dir)
	if errors.Is(err, fs.ErrNotExist) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	defer d.Close()
	var out []segInfo
	// Read the directory in chunks and stop as soon as the cap is passed, so a
	// directory with millions of entries is never listed whole.
	for {
		entries, err := d.ReadDir(256)
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
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			return nil, err
		}
	}
	// Directory order is arbitrary; segment order is (sequence, name).
	sort.Slice(out, func(i, j int) bool {
		if out[i].seq != out[j].seq {
			return out[i].seq < out[j].seq
		}
		return out[i].name < out[j].name
	})
	return out, nil
}

// readRecordFile reads one record file.  The path is Lstat-ed first, which
// refuses a symlink or Windows reparse point outright (where O_NOFOLLOW does
// not exist); then the file is opened and its type is checked again on the open
// descriptor, because a stat of the path alone leaves a TOCTOU window.  A file
// larger than budget bytes is refused before it is parsed.  It returns the
// records and the file's size.
func readRecordFile(path string, budget int64) ([]Record, int64, error) {
	name := filepath.Base(path)
	li, err := os.Lstat(path)
	if err != nil {
		return nil, 0, err
	}
	if !li.Mode().IsRegular() {
		return nil, 0, fmt.Errorf("ledger: %s is not a regular file", name)
	}
	f, err := openRead(path)
	if err != nil {
		return nil, 0, err
	}
	defer f.Close()
	size, err := requireRegular(f)
	if err != nil {
		return nil, 0, err
	}
	if size > budget {
		return nil, 0, fmt.Errorf("ledger: %s exceeds the %d byte read budget", name, budget)
	}
	recs, err := ParseSegment(f)
	if err != nil {
		return nil, 0, fmt.Errorf("ledger: %s: %w", name, err)
	}
	return recs, size, nil
}

// requireRegular checks the open descriptor's type and returns its size.
func requireRegular(f *os.File) (int64, error) {
	info, err := f.Stat()
	if err != nil {
		return 0, err
	}
	if !info.Mode().IsRegular() {
		return 0, fmt.Errorf("ledger: %s is not a regular file", filepath.Base(f.Name()))
	}
	return info.Size(), nil
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
// records' At times first.  If At is equal and the kinds differ, the TOMBSTONE
// wins (a false pass is the dangerous error, a miss is cheap), before any
// positional tie-break.  Remaining ties go to the higher segment sequence, then
// the lexically higher segment name, then the later line.  Nothing else (file
// modification time, directory listing order) is consulted, so the result is
// the same on every machine and every filesystem.
func (a entry) newer(b entry) bool {
	switch {
	case !a.rec.At.Equal(b.rec.At):
		return a.rec.At.After(b.rec.At)
	case a.rec.Kind != b.rec.Kind:
		return a.rec.Kind == KindTombstone
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
	return loadDirLimits(dir, MaxDirBytes, MaxDirRecords)
}

// loadDirLimits is loadDir with injectable cumulative caps on bytes and
// records, so tests need not build hundreds of megabytes.
func loadDirLimits(dir string, maxBytes int64, maxRecords int) ([]entry, error) {
	segs, err := listSegments(dir)
	if err != nil {
		return nil, err
	}
	var all []entry
	budget := maxBytes
	for _, s := range segs {
		recs, size, err := readRecordFile(filepath.Join(dir, s.name), min(budget, MaxSegmentBytes))
		if err != nil {
			return nil, err
		}
		budget -= size
		if len(all)+len(recs) > maxRecords {
			return nil, fmt.Errorf("ledger: more than %d records in %s", maxRecords, dir)
		}
		all = slices.Grow(all, len(recs))
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

// Record appends records as one NEW segment.  auth is the Host's verdict
// (KILN00 section 10.2): a nil Authority, or one that does not permit this
// Store's scope, writes nothing and yields ErrUntrusted.  Ordering trusts each
// record's At, so a record dated more than MaxClockSkew after now is refused.
// Every record is validated first and the batch is refused whole if any is
// invalid or the batch exceeds the segment caps.  An empty batch writes nothing.
func (s *Store) Record(records []Record, auth Authority, now time.Time) error {
	if err := authorize(auth, s.scope); err != nil {
		return err
	}
	if len(records) == 0 {
		return nil
	}
	for i, r := range records {
		if r.At.After(now.Add(MaxRecordSkew)) {
			return fmt.Errorf("ledger: record %d is dated %s, more than %s after now", i+1,
				r.At.Format(time.RFC3339), MaxRecordSkew)
		}
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
	rnd, err := randHexString(randHex)
	if err != nil {
		return err
	}
	return writeAtomic(s.fs, s.dir, segmentName(next, rnd), data)
}

// randHexString returns n random lowercase hex characters (n even).
func randHexString(n int) (string, error) {
	b := make([]byte, n/2)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}

// tmpFile is the part of *os.File that writeAtomic uses.
type tmpFile interface {
	io.Writer
	Sync() error
	Close() error
	Name() string
}

// fsOps is the seam tests use to inject failures.
type fsOps struct {
	create  func(dir string) (tmpFile, error)
	rename  func(oldpath, newpath string) error
	syncDir func(dir string) error
}

var realFS = fsOps{
	create:  func(dir string) (tmpFile, error) { return os.CreateTemp(dir, ".tmp-*") },
	rename:  os.Rename,
	syncDir: syncDir,
}

// writeAtomic makes name appear in dir complete or not at all: write to a temp
// file in the SAME directory (so the rename stays on one filesystem), fsync it,
// rename it into place, then fsync the directory so the rename survives a
// crash; a failed directory sync is an error.  A reader therefore never sees a
// partial segment; a crash leaves at most a ".tmp-*" file, which listSegments
// ignores.  On any failure before the rename the temp file is removed.
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
	return o.syncDir(dir)
}
