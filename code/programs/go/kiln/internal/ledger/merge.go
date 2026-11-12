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
//   - The delta is validated strictly BEFORE anything is written: one invalid
//     record rejects the whole delta (a half-trusted delta is not trusted).
//     The snapshot is validated just as strictly.  Both are read with the
//     segment caps, never unbounded.
//   - Compaction keeps ONE record per key: the newest by the ordering rule of
//     entry.newer.  The delta counts as newer than every snapshot segment for
//     tie-breaking.  Keeping the newest tombstone (rather than dropping it once
//     the older pass is gone) is what stops a stale delta from reviving a
//     revoked key later, since the stale pass's At is older than the tombstone.
//   - prune, if non-nil, drops winning records it selects (typically
//     OlderThan).  A tombstone is NEVER dropped while some pass for its key
//     survives the predicate: that tombstone still shadows a pass.  (With
//     OlderThan this cannot arise, because every pass a tombstone shadows is
//     older than the tombstone; it matters for other predicates.)
//   - Output is deterministic: records sorted by key, split into segments of at
//     most MaxRecordsPerSegment, named from the content, so merging the same
//     inputs twice yields byte-identical files.
//
// outDir must not exist or must be empty, and must differ from snapshotDir, so
// a failed merge can never damage the snapshot it started from.
func Merge(snapshotDir, deltaFile, outDir string, prune func(Record) bool) (MergeStats, error) {
	var st MergeStats
	if err := checkOutDir(snapshotDir, outDir); err != nil {
		return st, err
	}
	snap, err := loadDir(snapshotDir)
	if err != nil {
		return st, fmt.Errorf("snapshot: %w", err)
	}
	deltaRecs, err := readRecordFile(deltaFile)
	if err != nil {
		return st, fmt.Errorf("delta: %w", err)
	}
	st.Snapshot, st.Delta = len(snap), len(deltaRecs)
	all := snap
	for i, r := range deltaRecs {
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
	var out []Record
	for k, e := range newest(all) {
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

	if err := os.MkdirAll(outDir, 0o755); err != nil {
		return st, err
	}
	// The new snapshot continues the sequence, so "newest snapshot" stays
	// meaningful by sequence alone.
	var next uint64 = 1
	segs, _ := listSegments(snapshotDir) // already validated by loadDir
	if n := len(segs); n > 0 {
		next = segs[n-1].seq + 1
	}
	for len(out) > 0 {
		n := min(len(out), MaxRecordsPerSegment)
		data, err := EncodeSegment(out[:n])
		if err != nil {
			return st, err
		}
		if next > maxSeq {
			return st, errors.New("ledger: segment sequence exhausted")
		}
		sum := sha256.Sum256(data)
		name := segmentName(next, hex.EncodeToString(sum[:randHex/2]))
		if err := writeAtomic(realFS, outDir, name, data); err != nil {
			return st, err
		}
		out, next = out[n:], next+1
	}
	return st, nil
}

// checkOutDir refuses an output directory that is the snapshot itself or that
// already holds anything.
func checkOutDir(snapshotDir, outDir string) error {
	a, errA := filepath.Abs(snapshotDir)
	b, errB := filepath.Abs(outDir)
	if errA != nil || errB != nil {
		return errors.New("ledger: cannot resolve merge directories")
	}
	if a == b {
		return errors.New("ledger: output directory must differ from the snapshot directory")
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
