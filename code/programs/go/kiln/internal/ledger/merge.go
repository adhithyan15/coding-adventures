package ledger

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"sort"
	"time"
)

// MergeStats says what Merge did.
type MergeStats struct {
	Snapshot int // records read from the snapshot
	Delta    int // records read from the delta
	Kept     int // records written to the new snapshot
	Pruned   int // winning records dropped by the prune predicate
}

// OlderThan returns a prune predicate that selects records whose At is before
// cutoff.  Callers compute the cutoff ("now minus N days") so Merge itself
// never reads the clock and stays deterministic.
func OlderThan(cutoff time.Time) func(Record) bool {
	return func(r Record) bool { return r.At.Before(cutoff) }
}

// Merge is the core of the ledger-merge job (KILN00 section 10.2): it unions a
// trusted run's delta file into the newest snapshot directory and writes the
// result as a new, compacted snapshot in outDir.
//
// TRUST.  auth must permit shared-ledger writes (Authority.Trusted()).  Merge
// ASSUMES the delta artifact came from a run the Host marked trusted; it cannot
// check that itself.  A compromised trusted job can therefore still forge
// passes.  Three things bound the damage: the delta's timestamps are
// overwritten (below), so a forger cannot outrank a tombstone; the daily
// uncached run finds a false pass and writes a tombstone; and tombstones win
// ties.
//
//   - The delta is validated strictly BEFORE anything is written: one invalid
//     record rejects the whole delta (a half-trusted delta is not trusted).
//     The snapshot is validated just as strictly.  Both are read with the
//     segment caps, never unbounded.
//   - RE-STAMPING.  Ordering trusts At, so a writer must not choose its own
//     time relative to tombstones.  A delta pass keeps its At, capped at now
//     (so a stale pass stays behind a newer tombstone); a delta tombstone's At
//     becomes now (so it outranks every earlier pass).  A delta record dated
//     more than MaxClockSkew after now rejects the delta.  now is UTC, whole
//     seconds.  now is a parameter, never read from the clock, so
//     the output stays deterministic.  A snapshot record dated more than
//     MaxClockSkew after now is an error (fail closed): it is either forged or
//     the clock is badly wrong.
//   - Compaction keeps ONE record per key: the newest by the ordering rule of
//     entry.newer (equal At: tombstone wins; the delta outranks the snapshot).
//     Keeping the newest tombstone (rather than dropping it once the older
//     pass is gone) stops a stale delta from reviving a revoked key later.
//   - prune, if non-nil, drops winning records it selects (typically
//     OlderThan).  A tombstone is NEVER dropped while some pass for its key
//     survives the predicate: that tombstone still shadows a pass.  (With
//     OlderThan this cannot arise, because every pass a tombstone shadows is
//     older than the tombstone; it matters for other predicates.)
//   - Output is deterministic: records sorted by key, split into segments of at
//     most MaxRecordsPerSegment, named from the content, so merging the same
//     inputs twice yields byte-identical files.
//
// outDir must not exist or must be empty, and must differ from snapshotDir
// (after resolving symlinks), so a failed merge can never damage the snapshot
// it started from.  The new snapshot is built in a sibling directory
// "<outDir>.partial-<random>" and renamed into place at the end, so outDir is
// either absent/empty or complete; the partial directory is removed on error.
func Merge(snapshotDir, deltaFile, outDir string, auth Authority, now time.Time, prune func(Record) bool) (MergeStats, error) {
	return merge(realFS, snapshotDir, deltaFile, outDir, auth, now, prune)
}

func merge(o fsOps, snapshotDir, deltaFile, outDir string, auth Authority, now time.Time, prune func(Record) bool) (st MergeStats, err error) {
	if err := authorize(auth, Shared); err != nil {
		return st, err
	}
	outDir = filepath.Clean(outDir) // "out/" and "out" must name one directory
	now = now.UTC().Truncate(time.Second)
	if now.IsZero() || now.Year() > 9999 {
		return st, errors.New("ledger: merge needs a valid now")
	}
	if err := checkOutDir(snapshotDir, outDir); err != nil {
		return st, err
	}
	snap, err := loadDir(snapshotDir)
	if err != nil {
		return st, fmt.Errorf("snapshot: %w", err)
	}
	for _, e := range snap {
		if e.rec.At.After(now.Add(MaxClockSkew)) {
			return st, fmt.Errorf("snapshot: %s: record for %s is dated %s, after now",
				e.name, e.rec.Hex(), e.rec.At.Format(time.RFC3339))
		}
	}
	deltaRecs, _, err := readRecordFile(deltaFile, MaxSegmentBytes)
	if err != nil {
		return st, fmt.Errorf("delta: %w", err)
	}
	st.Snapshot, st.Delta = len(snap), len(deltaRecs)
	all := make([]entry, len(snap), len(snap)+len(deltaRecs))
	copy(all, snap)
	for i, r := range deltaRecs {
		if r.At.After(now.Add(MaxClockSkew)) {
			return st, fmt.Errorf("delta: line %d is dated %s, after now", i+1, r.At.Format(time.RFC3339))
		}
		// Re-stamp: the writer does not choose its own time.  A pass keeps
		// its own (earlier) time, capped at now, so a job that started before
		// a tombstone cannot undo it by merging after it; a tombstone is
		// stamped now, so it outranks every pass from before the merge.
		if r.Kind == KindTombstone || r.At.After(now) {
			r.At = now
		}
		all = append(all, entry{r, math.MaxUint64, "", i + 1})
	}

	// survivingPass[k]: some pass for k is not selected by prune.
	survivingPass := map[[32]byte]bool{}
	if prune != nil {
		for _, e := range all {
			if e.rec.Kind == KindPass && !prune(e.rec) {
				survivingPass[e.rec.Key] = true
			}
		}
	}
	best := newest(all)
	out := make([]Record, 0, len(best))
	for k, e := range best {
		drop := prune != nil && prune(e.rec)
		if drop && e.rec.Kind == KindTombstone && survivingPass[k] {
			drop = false
		}
		if drop {
			st.Pruned++
			continue
		}
		out = append(out, e.rec)
	}
	sort.Slice(out, func(i, j int) bool { return bytes.Compare(out[i].Key[:], out[j].Key[:]) < 0 })
	st.Kept = len(out)

	// The new snapshot continues the sequence, so "newest snapshot" stays
	// meaningful by sequence alone.
	var next uint64 = 1
	segs, _ := listSegments(snapshotDir) // already validated by loadDir
	if n := len(segs); n > 0 {
		next = segs[n-1].seq + 1
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
	for len(out) > 0 {
		n := min(len(out), MaxRecordsPerSegment)
		data, encErr := EncodeSegment(out[:n])
		if encErr != nil {
			return st, encErr
		}
		if next > maxSeq {
			return st, errors.New("ledger: segment sequence exhausted")
		}
		sum := sha256.Sum256(data)
		name := segmentName(next, hex.EncodeToString(sum[:randHex/2]))
		if err = writeAtomic(o, partial, name, data); err != nil {
			return st, err
		}
		out, next = out[n:], next+1
	}
	// An existing empty outDir is removed so the rename also works on Windows.
	// If it is not empty after all, the rename below fails and nothing is lost.
	_ = os.Remove(outDir)
	if err = o.rename(partial, outDir); err != nil {
		return st, err
	}
	err = o.syncDir(filepath.Dir(outDir))
	return st, err
}

// resolvePath makes p absolute and resolves symlinks in its longest existing
// prefix, so two spellings of one directory compare equal even when the
// directory itself does not exist yet.
func resolvePath(p string) (string, error) {
	abs, err := filepath.Abs(p)
	if err != nil {
		return "", err
	}
	rest := ""
	for cur := abs; ; {
		if r, err := filepath.EvalSymlinks(cur); err == nil {
			return filepath.Join(r, rest), nil
		}
		parent := filepath.Dir(cur)
		if parent == cur { // reached the root without finding anything
			return abs, nil
		}
		rest = filepath.Join(filepath.Base(cur), rest)
		cur = parent
	}
}

// checkOutDir refuses an output directory that is the snapshot itself (under
// any alias) or that already holds anything.
func checkOutDir(snapshotDir, outDir string) error {
	a, errA := resolvePath(snapshotDir)
	b, errB := resolvePath(outDir)
	if errA != nil || errB != nil {
		return errors.New("ledger: cannot resolve merge directories")
	}
	if a == b {
		return errors.New("ledger: output directory must differ from the snapshot directory")
	}
	if err := checkPlainDir(outDir); err != nil { // ReadDir would follow a symlink
		return err
	}
	entries, err := os.ReadDir(outDir)
	switch {
	case os.IsNotExist(err):
		return nil
	case err != nil:
		return err
	case len(entries) > 0:
		return errors.New("ledger: output directory is not empty")
	}
	return nil
}
