package ledger

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The high-water mark (KILN00 section 18 Q9).
//
// A ledger snapshot is a cache artifact. An attacker who can restore an older
// one resurrects the revoked passes it still contains, because a tombstone only
// revokes the passes in the snapshots that came after it. The mark is the
// newest sequence this ledger has already merged, recorded OUTSIDE the
// snapshot, so rolling the snapshot back cannot roll the mark back with it.

// mergedSnapshot runs one real merge and returns the output directory. The
// output's newest sequence is what a caller records as its mark.
func mergedSnapshot(t *testing.T, snap string, delta ...Record) string {
	t.Helper()
	out := filepath.Join(t.TempDir(), "out")
	if _, err := Merge(snap, deltaFile(t, delta...), out, trusted, nowT, nil); err != nil {
		t.Fatalf("Merge: %v", err)
	}
	return out
}

func TestMergeRefusesASnapshotOlderThanTheMark(t *testing.T) {
	// A snapshot whose newest sequence is 1, and a mark of 2: the previous
	// merge had already written sequence 2, so this snapshot is a rollback.
	snap := snapshotWith(t, pass(1, t0))
	if newest, err := NewestSequence(snap); err != nil || newest != 1 {
		t.Fatalf("NewestSequence = %d, %v; want 1", newest, err)
	}
	out := filepath.Join(t.TempDir(), "out")
	_, err := MergeWithOptions(MergeOptions{
		SnapshotDir: snap,
		DeltaFile:   deltaFile(t, pass(2, t0)),
		OutDir:      out,
		Auth:        trusted,
		Now:         nowT,
		HighWater:   2,
	})
	if err == nil {
		t.Fatal("a snapshot older than the mark was merged")
	}
	if !strings.Contains(err.Error(), "older than the high-water mark") {
		t.Errorf("error does not name the mark: %v", err)
	}
	// Refusal must be fail-closed: nothing written, not even the partial dir.
	if _, statErr := os.Stat(out); !errors.Is(statErr, os.ErrNotExist) {
		t.Errorf("a refused merge wrote %s", out)
	}
}

func TestMergeAcceptsASnapshotAtTheMark(t *testing.T) {
	// The normal case: the last merge wrote sequence 1, and the caller passes
	// that back as the mark. The next merge must accept it and continue at 2.
	snap := snapshotWith(t, pass(1, t0))
	out := filepath.Join(t.TempDir(), "out")
	if _, err := MergeWithOptions(MergeOptions{
		SnapshotDir: snap,
		DeltaFile:   deltaFile(t, pass(2, t0)),
		OutDir:      out,
		Auth:        trusted,
		Now:         nowT,
		HighWater:   1,
	}); err != nil {
		t.Fatalf("Merge at the mark: %v", err)
	}
	if newest, err := NewestSequence(out); err != nil || newest != 2 {
		t.Fatalf("NewestSequence(out) = %d, %v; want 2", newest, err)
	}
}

func TestMergeWithoutAMarkAcceptsAnySnapshot(t *testing.T) {
	// A first merge has no mark, and must accept whatever snapshot it finds.
	snap := snapshotWith(t, pass(1, t0))
	out := filepath.Join(t.TempDir(), "out")
	if _, err := MergeWithOptions(MergeOptions{
		SnapshotDir: snap,
		DeltaFile:   deltaFile(t, pass(2, t0)),
		OutDir:      out,
		Auth:        trusted,
		Now:         nowT,
	}); err != nil {
		t.Fatalf("Merge without a mark: %v", err)
	}
}

func TestMergeRefusesAnEmptySnapshotWhenTheMarkIsSet(t *testing.T) {
	// An absent snapshot is a valid first merge, but not when the mark says
	// this ledger has already merged. That is a rollback to "nothing".
	empty := filepath.Join(t.TempDir(), "empty")
	out := filepath.Join(t.TempDir(), "out")
	_, err := MergeWithOptions(MergeOptions{
		SnapshotDir: empty,
		DeltaFile:   deltaFile(t, pass(1, t0)),
		OutDir:      out,
		Auth:        trusted,
		Now:         nowT,
		HighWater:   1,
	})
	if err == nil {
		t.Fatal("an empty snapshot was merged under a mark")
	}
	if !strings.Contains(err.Error(), "older than the high-water mark") {
		t.Errorf("error does not name the mark: %v", err)
	}
}

func TestNewestSequenceOfAnAbsentDirectory(t *testing.T) {
	// The mark of a ledger that has never merged. A caller must be able to
	// read it without special-casing a missing directory.
	seq, err := NewestSequence(filepath.Join(t.TempDir(), "absent"))
	if err != nil {
		t.Fatalf("NewestSequence: %v", err)
	}
	if seq != 0 {
		t.Errorf("NewestSequence = %d, want 0", seq)
	}
}

func TestHighWaterMarkRoundTripsThroughAMerge(t *testing.T) {
	// The property the merge job depends on: read the mark after a merge, pass
	// it to the next merge, and the next merge accepts. Then roll the snapshot
	// back one generation and the next merge refuses.
	first := mergedSnapshot(t, snapshotWith(t, pass(1, t0)), pass(2, t0))
	mark, err := NewestSequence(first)
	if err != nil {
		t.Fatalf("NewestSequence: %v", err)
	}
	if mark == 0 {
		t.Fatal("NewestSequence returned 0 after a merge")
	}

	second := filepath.Join(t.TempDir(), "second")
	if _, err := MergeWithOptions(MergeOptions{
		SnapshotDir: first,
		DeltaFile:   deltaFile(t, pass(3, t0)),
		OutDir:      second,
		Auth:        trusted,
		Now:         nowT,
		HighWater:   mark,
	}); err != nil {
		t.Fatalf("the merge after reading the mark refused: %v", err)
	}

	// Rolling back to the FIRST snapshot is now detectable: the mark is at
	// least the newest sequence of the second merge's input.
	rolledBack := filepath.Join(t.TempDir(), "rollback")
	_, err = MergeWithOptions(MergeOptions{
		SnapshotDir: first,
		DeltaFile:   deltaFile(t, pass(4, t0)),
		OutDir:      rolledBack,
		Auth:        trusted,
		Now:         nowT,
		HighWater:   mark + 1,
	})
	if err == nil {
		t.Fatal("a rolled-back snapshot was merged")
	}
}

func TestMergeStatsAreUnchangedByTheMark(t *testing.T) {
	// The mark is a gate, not a filter: a merge that passes it must produce
	// exactly the same bytes as one that had no mark.
	snap := snapshotWith(t, pass(1, t0), pass(2, t0))
	delta := deltaFile(t, pass(3, t0), tomb(1, t0))

	plain := filepath.Join(t.TempDir(), "plain")
	stPlain, err := Merge(snap, delta, plain, trusted, nowT, nil)
	if err != nil {
		t.Fatalf("Merge: %v", err)
	}
	marked := filepath.Join(t.TempDir(), "marked")
	stMarked, err := MergeWithOptions(MergeOptions{
		SnapshotDir: snap,
		DeltaFile:   delta,
		OutDir:      marked,
		Auth:        trusted,
		Now:         nowT,
		HighWater:   1,
	})
	if err != nil {
		t.Fatalf("MergeWithOptions: %v", err)
	}
	if stPlain != stMarked {
		t.Errorf("stats differ: %+v vs %+v", stPlain, stMarked)
	}
	if dirBytes(t, plain) != dirBytes(t, marked) {
		t.Error("the marked merge wrote different bytes")
	}
}

func TestMergeRefusesAnUntrustedWriterBeforeTheMark(t *testing.T) {
	// Authority is checked first: an untrusted caller must not learn whether
	// the snapshot is stale, and must not be able to probe the mark.
	snap := snapshotWith(t, pass(1, t0))
	_, err := MergeWithOptions(MergeOptions{
		SnapshotDir: snap,
		DeltaFile:   deltaFile(t, pass(2, t0)),
		OutDir:      filepath.Join(t.TempDir(), "out"),
		Auth:        fakeAuth{},
		Now:         nowT,
		HighWater:   99,
	})
	if !errors.Is(err, ErrUntrusted) {
		t.Errorf("err = %v, want ErrUntrusted", err)
	}
}

func TestNewestSequenceRefusesANonRegularSegment(t *testing.T) {
	// A rollback check that trusts a directory listing is only as good as the
	// listing. A non-regular segment must be refused, as loadDir does.
	dir := t.TempDir()
	if err := os.Mkdir(filepath.Join(dir, segmentName(1, "0123456789abcdef")), 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := NewestSequence(dir); err == nil {
		t.Error("NewestSequence accepted a directory named like a segment")
	}
}
