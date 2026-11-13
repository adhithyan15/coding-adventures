package ledger

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"math"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"time"
)

// ManifestDrop says what a reader dropped, and why.  Line 0 means the whole
// segment.
type ManifestDrop struct {
	Segment string
	Line    int
	Reason  string
}

// ManifestReport is what a read did besides returning manifests.  It is the
// "dropped and reported" half of the reader policy.
type ManifestReport struct {
	Segments   int            // segment files seen
	Read       int            // records accepted from segments (before retention)
	Dropped    []ManifestDrop // over-cap records and segments
	Expired    int            // accepted records older than ManifestMaxAge
	Future     int            // accepted records dated after now plus MaxClockSkew (dropped; also listed in Dropped)
	Superseded int            // accepted records beyond the newest K distinct ids, or repeating an id
	Retained   int            // records kept
}

// manifestLimits are the segment-level caps, injectable so tests need not
// build 64 MiB files.
type manifestLimits struct {
	line    int
	segment int64
	store   int64
}

var defaultManifestLimits = manifestLimits{MaxManifestLineBytes, MaxManifestSegmentBytes, MaxManifestStoreBytes}

// READER POLICY (KILN04 section 8).  A manifest is store-only: it can never
// cause a hit by itself, so losing one only costs time.  The reader therefore
// distinguishes two failures:
//
//   - OVER-CAP (a record over 8,192 entries, 512 bytes per name or 1 MiB per
//     line, or a segment over 64 MiB): drop just that record or segment, and
//     report it.  The rest of the store is still good.
//   - STRUCTURAL CORRUPTION (a non-canonical line, an unknown field, a bad
//     entry or target, an id that does not match the entries, a missing final
//     newline, a segment that is not a regular file, or a store over 256 MiB in
//     total): the whole store is UNUSABLE.  An error is returned and the
//     caller gets no manifests, so no hits.  Nothing is guessed or repaired,
//     because a writer that produced one such line may have produced others
//     that merely look fine.
//
// A line longer than 1 MiB is dropped without being parsed.

// readManifestFile reads one segment under lim.  tooBig means the file is over
// the segment cap (it is not read).  size is the file's size either way, so
// the caller can charge it to the store budget.
func readManifestFile(path string, lim manifestLimits) (data []byte, size int64, tooBig bool, err error) {
	name := filepath.Base(path)
	li, err := os.Lstat(path)
	if err != nil {
		return nil, 0, false, err
	}
	if !li.Mode().IsRegular() {
		return nil, 0, false, fmt.Errorf("ledger: manifest segment %s is not a regular file", name)
	}
	f, err := openRead(path)
	if err != nil {
		return nil, 0, false, err
	}
	defer f.Close()
	size, err = requireRegular(f)
	if err != nil {
		return nil, 0, false, err
	}
	if size > lim.segment {
		return nil, size, true, nil
	}
	// Size the buffer from the stat so the read does not regrow it; the
	// LimitReader still bounds what a file that grows meanwhile can add.
	var buf bytes.Buffer
	buf.Grow(int(size) + 1)
	if _, err = buf.ReadFrom(io.LimitReader(f, lim.segment+1)); err != nil {
		return nil, 0, false, err
	}
	data = buf.Bytes()
	if int64(len(data)) > lim.segment { // grew while we read it
		return nil, int64(len(data)), true, nil
	}
	return data, size, false, nil
}

// parseManifestSegment decodes one segment's bytes.  Over-cap records are
// appended to rep.Dropped; anything structural is an error.
func parseManifestSegment(name string, seq uint64, data []byte, lim manifestLimits, rep *ManifestReport) ([]mEntry, error) {
	var out []mEntry
	for n := 1; len(data) > 0; n++ {
		i := bytes.IndexByte(data, '\n')
		if i < 0 {
			return nil, fmt.Errorf("ledger: manifest segment %s line %d: missing final newline", name, n)
		}
		line := data[:i]
		data = data[i+1:]
		if len(line)+1 > lim.line {
			rep.Dropped = append(rep.Dropped, ManifestDrop{name, n,
				fmt.Sprintf("line is %d bytes, limit %d", len(line)+1, lim.line)})
			continue
		}
		m, err := decodeManifestLine(line)
		switch {
		case errors.Is(err, errOverCap):
			rep.Dropped = append(rep.Dropped, ManifestDrop{name, n, err.Error()})
			continue
		case err != nil:
			return nil, fmt.Errorf("ledger: manifest segment %s line %d: %w", name, n, err)
		}
		out = append(out, mEntry{m, seq, name, n})
	}
	return out, nil
}

// retainer applies retention INCREMENTALLY, so the heap holds at most
// ManifestKeep records per weak key (plus the segment being parsed) instead of
// every accepted record.  A record is discarded on arrival when it is older
// than ManifestMaxAge before now, or dated after now plus MaxClockSkew (a
// far-future record would never expire and would outrank real ones forever);
// otherwise it competes for one of the weak key's K slots, which hold DISTINCT
// ids, each with its newest instance.  A repeated id (a scheduled re-record)
// therefore never uses a second slot.  The outcome equals sorting everything
// and cutting afterwards: the order is total, and an id's newest instance is
// the only one that can matter.
type retainer struct {
	now        time.Time
	failFuture bool // a far-future record is an error (merge snapshots), not a drop
	by         map[[32]byte][]mEntry
	seen       int
	expired    int
	drops      []ManifestDrop // far-future records
}

func newRetainer(now time.Time, failFuture bool) *retainer {
	return &retainer{now: now.UTC(), failFuture: failFuture, by: map[[32]byte][]mEntry{}}
}

func (r *retainer) add(e mEntry) error {
	r.seen++
	if e.m.At.After(r.now.Add(MaxClockSkew)) {
		if r.failFuture {
			return fmt.Errorf("ledger: manifest segment %s line %d is dated %s, after now",
				e.name, e.line, e.m.At.Format(time.RFC3339))
		}
		r.drops = append(r.drops, ManifestDrop{e.name, e.line,
			"dated " + e.m.At.Format(time.RFC3339) + ", after now plus the clock skew"})
		return nil
	}
	if e.m.At.Before(r.now.Add(-ManifestMaxAge)) {
		r.expired++
		return nil
	}
	w, id := e.m.Weak, e.m.ID()
	list := r.by[w]
	for i := range list {
		if list[i].m.ID() == id {
			if e.newer(list[i]) {
				list[i] = e
				sortNewestFirst(list)
			}
			return nil
		}
	}
	list = append(list, e)
	sortNewestFirst(list)
	if len(list) > ManifestKeep {
		list = list[:ManifestKeep]
	}
	r.by[w] = list
	return nil
}

func sortNewestFirst(l []mEntry) {
	sort.Slice(l, func(i, j int) bool { return l[i].newer(l[j]) })
}

// finish fills the report's retention counters and returns the retained
// records per weak key, newest first.
func (r *retainer) finish(rep *ManifestReport) map[[32]byte][]mEntry {
	retained := 0
	for _, l := range r.by {
		retained += len(l)
	}
	rep.Expired, rep.Future, rep.Retained = r.expired, len(r.drops), retained
	rep.Superseded = r.seen - r.expired - len(r.drops) - retained
	rep.Dropped = append(rep.Dropped, r.drops...)
	return r.by
}

// loadManifestDir reads every segment of dir under the reader policy above and
// feeds each accepted record to acc as it goes.
func loadManifestDir(dir string, lim manifestLimits, acc *retainer) (ManifestReport, error) {
	var rep ManifestReport
	segs, err := listSegments(dir)
	if err != nil {
		return rep, err
	}
	var total int64
	for _, s := range segs {
		rep.Segments++
		data, size, tooBig, err := readManifestFile(filepath.Join(dir, s.name), lim)
		if err != nil {
			return rep, err
		}
		if size > lim.store-total { // this form cannot overflow
			return rep, fmt.Errorf("ledger: manifest store exceeds %d bytes", lim.store)
		}
		total += size
		if tooBig {
			rep.Dropped = append(rep.Dropped, ManifestDrop{s.name, 0,
				fmt.Sprintf("segment is %d bytes, limit %d", size, lim.segment)})
			continue
		}
		es, err := parseManifestSegment(s.name, s.seq, data, lim, &rep)
		if err != nil {
			return rep, err
		}
		rep.Read += len(es)
		for _, e := range es {
			if err := acc.add(e); err != nil {
				return rep, err
			}
		}
	}
	return rep, nil
}

// ManifestSet is a loaded, retained manifest store.
type ManifestSet struct {
	byWeak map[[32]byte][]mEntry
}

// Manifests returns the retained manifests for a weak key, newest first, at
// most ManifestKeep, all with distinct ids.  This is what legacy-plan calls
// (KILN04 section 3.3).  The caller owns the returned slice.
func (s *ManifestSet) Manifests(weak [32]byte) []Manifest {
	es := s.byWeak[weak]
	if len(es) == 0 {
		return nil
	}
	out := make([]Manifest, len(es))
	for i, e := range es {
		out[i] = e.m
		out[i].Entries = slices.Clone(e.m.Entries)
	}
	return out
}

// ManifestStore is the manifests/ directory of a ledger directory.
type ManifestStore struct {
	root  string // the ledger directory, checked to be a plain directory
	dir   string
	scope Scope
	fs    fsOps
}

// OpenManifests returns the manifest store of the ledger directory ledgerDir,
// that is, ledgerDir/manifests.  The directory need not exist.  The pass-ledger
// reader ignores it: "manifests" does not match the segment-name pattern.
func OpenManifests(ledgerDir string, scope Scope) *ManifestStore {
	return &ManifestStore{root: ledgerDir, dir: filepath.Join(ledgerDir, ManifestsDir), scope: scope, fs: realFS}
}

// Load reads the store under the reader policy and applies retention relative
// to now (a parameter, never the clock).  On an error (structural corruption,
// an over-cap store) the set is nil and the caller must treat it as no hits.
func (s *ManifestStore) Load(now time.Time) (*ManifestSet, ManifestReport, error) {
	if err := checkPlainDir(s.root); err != nil {
		return nil, ManifestReport{}, err
	}
	acc := newRetainer(now, false)
	rep, err := loadManifestDir(s.dir, defaultManifestLimits, acc)
	if err != nil {
		return nil, rep, err
	}
	return &ManifestSet{acc.finish(&rep)}, rep, nil
}

// packLines groups encoded lines into segments of at most maxBytes each.  A
// single line larger than maxBytes cannot be packed; encoded lines are capped
// well below that, so this only guards injected limits.
func packLines(lines [][]byte, maxBytes int64) ([][]byte, error) {
	var segs [][]byte
	var cur []byte
	for _, l := range lines {
		if int64(len(l)) > maxBytes {
			return nil, fmt.Errorf("ledger: a manifest line of %d bytes exceeds the segment limit %d", len(l), maxBytes)
		}
		if int64(len(cur)+len(l)) > maxBytes {
			segs = append(segs, cur)
			cur = nil
		}
		cur = append(cur, l...)
	}
	if len(cur) > 0 {
		segs = append(segs, cur)
	}
	return segs, nil
}

// Record appends manifests as one NEW segment.  The gate is the pass ledger's:
// a nil Authority, or one that does not permit this store's scope, writes
// nothing and yields ErrUntrusted (only trusted hosts write the shared store).
// Every manifest is validated first and the batch is refused whole if any is
// invalid or over a per-record cap (the error then wraps ErrManifestTooLarge,
// which the caller reports as `manifest-too-large` for that action) or if the
// batch does not fit one segment.  Manifests are not restamped; one dated more
// than MaxRecordSkew after now is refused.  An empty batch writes nothing.
func (s *ManifestStore) Record(ms []Manifest, auth Authority, now time.Time) error {
	if err := authorize(auth, s.scope); err != nil {
		return err
	}
	if len(ms) == 0 {
		return nil
	}
	lines := make([][]byte, len(ms))
	for i, m := range ms {
		if m.At.After(now.Add(MaxRecordSkew)) {
			return fmt.Errorf("ledger: manifest %d is dated %s, more than %s after now", i+1,
				m.At.Format(time.RFC3339), MaxRecordSkew)
		}
		l, err := m.Encode()
		if err != nil {
			return fmt.Errorf("manifest %d: %w", i+1, err)
		}
		lines[i] = l
	}
	segs, err := packLines(lines, MaxManifestSegmentBytes)
	if err != nil {
		return err
	}
	if len(segs) != 1 {
		return fmt.Errorf("ledger: %d manifests exceed the %d byte segment limit", len(ms), MaxManifestSegmentBytes)
	}
	// MkdirAll follows a symlinked directory silently, so check first.
	if err := checkPlainDir(s.root); err != nil {
		return err
	}
	if err := checkPlainDir(s.dir); err != nil {
		return err
	}
	if err := os.MkdirAll(s.dir, 0o755); err != nil {
		return err
	}
	existing, err := listSegments(s.dir)
	if err != nil {
		return err
	}
	var next uint64 = 1
	if n := len(existing); n > 0 {
		next = existing[n-1].seq + 1
	}
	if next > maxSeq {
		return errors.New("ledger: segment sequence exhausted")
	}
	rnd, err := randHexString(randHex)
	if err != nil {
		return err
	}
	return writeAtomic(s.fs, s.dir, segmentName(next, rnd), segs[0])
}

// ManifestMergeStats says what MergeManifests did.
type ManifestMergeStats struct {
	Snapshot   int            // records accepted from the snapshot
	Delta      int            // records read from the delta
	Kept       int            // records written to the new snapshot
	Expired    int            // older than ManifestMaxAge
	Superseded int            // beyond the newest K distinct ids, or a repeated id
	Dropped    []ManifestDrop // over-cap snapshot records and segments (not carried over)
}

// MergeManifests is the manifest half of the ledger-merge job: it unions a
// delta file into the snapshot manifest directory, applies retention, and
// writes the result as a new snapshot in outDir.
//
//   - TRUST.  auth must permit shared writes (Authority.Trusted()), as for
//     Merge.  The delta is assumed to come from a trusted run.
//   - The delta is read STRICTLY: any structural error, and also any over-cap
//     record or segment, rejects the whole delta (a writer must have refused
//     to emit it).  The snapshot is read under the reader policy: structural
//     corruption fails the merge; over-cap records are dropped and reported in
//     the stats (they are not carried into the new snapshot).
//   - Not restamped: a manifest keeps its own run and at.  A delta record
//     dated more than MaxClockSkew after now rejects the delta.
//   - Retention is retainManifests.  Output is deterministic: weak keys in byte
//     order, and within one weak key OLDEST FIRST, so that file position
//     (the last ordering tie-break) in the new snapshot agrees with the rank
//     that decided the merge.  Merging an unchanged snapshot with an empty
//     delta therefore reproduces it byte for byte.  Segments are packed up to
//     MaxManifestSegmentBytes and named from their content.
//
// outDir must not exist or must be empty and must differ from snapshotDir; the
// new snapshot is built in "<outDir>.partial-<random>" and renamed into place.
func MergeManifests(snapshotDir, deltaFile, outDir string, auth Authority, now time.Time) (ManifestMergeStats, error) {
	return mergeManifests(realFS, defaultManifestLimits, snapshotDir, deltaFile, outDir, auth, now)
}

func mergeManifests(o fsOps, lim manifestLimits, snapshotDir, deltaFile, outDir string, auth Authority, now time.Time) (st ManifestMergeStats, err error) {
	if err := authorize(auth, Shared); err != nil {
		return st, err
	}
	outDir = filepath.Clean(outDir)
	now = now.UTC().Truncate(time.Second)
	if now.IsZero() || now.Year() > 9999 {
		return st, errors.New("ledger: manifest merge needs a valid now")
	}
	if err := checkOutDir(snapshotDir, outDir); err != nil {
		return st, err
	}
	// A far-future snapshot record is an error (as in the pass Merge), not a
	// drop: it is forged or the clock is badly wrong.
	acc := newRetainer(now, true)
	snapRep, err := loadManifestDir(snapshotDir, lim, acc)
	if err != nil {
		return st, fmt.Errorf("snapshot: %w", err)
	}
	st.Snapshot, st.Dropped = snapRep.Read, snapRep.Dropped

	data, _, tooBig, err := readManifestFile(deltaFile, lim)
	if err != nil {
		return st, fmt.Errorf("delta: %w", err)
	}
	if tooBig {
		return st, fmt.Errorf("delta: larger than %d bytes", lim.segment)
	}
	var deltaRep ManifestReport
	delta, err := parseManifestSegment(filepath.Base(deltaFile), math.MaxUint64, data, lim, &deltaRep)
	if err != nil {
		return st, fmt.Errorf("delta: %w", err)
	}
	if len(deltaRep.Dropped) > 0 {
		d := deltaRep.Dropped[0]
		return st, fmt.Errorf("delta: line %d: %s", d.Line, d.Reason)
	}
	st.Delta = len(delta)
	for _, e := range delta {
		if e.m.At.After(now.Add(MaxClockSkew)) {
			return st, fmt.Errorf("delta: line %d is dated %s, after now", e.line, e.m.At.Format(time.RFC3339))
		}
		e.name = "" // the delta outranks every snapshot segment on a positional tie
		if err := acc.add(e); err != nil {
			return st, err
		}
	}

	var rep ManifestReport
	kept := acc.finish(&rep)
	st.Expired, st.Superseded, st.Kept = rep.Expired, rep.Superseded, rep.Retained

	weaks := make([][32]byte, 0, len(kept))
	for w := range kept {
		weaks = append(weaks, w)
	}
	sort.Slice(weaks, func(i, j int) bool { return bytes.Compare(weaks[i][:], weaks[j][:]) < 0 })
	var lines [][]byte
	for _, w := range weaks {
		es := kept[w]
		for i := len(es) - 1; i >= 0; i-- { // oldest first
			lines = append(lines, es[i].m.encode())
		}
	}
	segs, err := packLines(lines, lim.segment)
	if err != nil {
		return st, err
	}

	var next uint64 = 1
	existing, _ := listSegments(snapshotDir) // already validated by loadManifestDir
	if n := len(existing); n > 0 {
		next = existing[n-1].seq + 1
	}
	rnd, err := randHexString(randHex)
	if err != nil {
		return st, err
	}
	partial := outDir + ".partial-" + rnd
	if err := os.MkdirAll(partial, 0o755); err != nil {
		return st, err
	}
	defer func() {
		if err != nil {
			_ = os.RemoveAll(partial) // our own scratch directory
		}
	}()
	for _, seg := range segs {
		if next > maxSeq {
			return st, errors.New("ledger: segment sequence exhausted")
		}
		sum := sha256.Sum256(seg)
		name := segmentName(next, hex.EncodeToString(sum[:randHex/2]))
		if err = writeAtomic(o, partial, name, seg); err != nil {
			return st, err
		}
		next++
	}
	_ = os.Remove(outDir) // an empty outDir, so the rename also works on Windows
	if err = o.rename(partial, outDir); err != nil {
		return st, err
	}
	err = o.syncDir(filepath.Dir(outDir))
	return st, err
}
