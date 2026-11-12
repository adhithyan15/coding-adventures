package ledger

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// Tests for the trust, clock-skew, ordering and filesystem hardening added
// after the security review.

func TestAuthorityAndScope(t *testing.T) {
	local := fakeAuth{local: true}
	cases := []struct {
		scope Scope
		auth  Authority
		ok    bool
	}{
		{Shared, trusted, true},
		{Shared, local, false},
		{Shared, fakeAuth{}, false},
		{Shared, nil, false},
		{Local, local, true},
		{Local, trusted, false},
		{Local, fakeAuth{}, false},
		{Local, nil, false},
		{Scope(0), trusted, false},
	}
	for i, c := range cases {
		dir := t.TempDir()
		err := Open(dir, c.scope).Record([]Record{pass(1, t0)}, c.auth, nowT)
		if c.ok != (err == nil) || (!c.ok && !errors.Is(err, ErrUntrusted)) {
			t.Errorf("case %d: ok=%v err=%v", i, c.ok, err)
		}
		if ents, _ := os.ReadDir(dir); !c.ok && len(ents) != 0 {
			t.Errorf("case %d: refused write left files", i)
		}
	}
	// Merge needs Trusted() too.
	for _, a := range []Authority{nil, fakeAuth{}, local} {
		out := filepath.Join(t.TempDir(), "o")
		if _, err := Merge(snapshotWith(t), deltaFile(t, pass(1, t0)), out, a, nowT, nil); !errors.Is(err, ErrUntrusted) {
			t.Errorf("merge with %v: %v", a, err)
		}
	}
}

func TestFormatCharactersRejected(t *testing.T) {
	for _, s := range []string{"a‮b", "a​b", "a⁦b"} {
		r := pass(1, t0)
		r.Target = s
		if err := r.Validate(); err == nil {
			t.Errorf("%q accepted", s)
		}
		r = pass(1, t0)
		r.Revision = s
		if err := r.Validate(); err == nil {
			t.Errorf("revision %q accepted", s)
		}
	}
}

func TestRecordClockSkew(t *testing.T) {
	s := Open(t.TempDir(), Shared)
	if err := s.Record([]Record{pass(1, nowT.Add(MaxClockSkew))}, trusted, nowT); err != nil {
		t.Fatalf("at the boundary: %v", err)
	}
	if err := s.Record([]Record{pass(2, nowT.Add(MaxClockSkew+time.Second))}, trusted, nowT); err == nil {
		t.Fatal("one second past the boundary accepted")
	}
	far := time.Date(9999, 1, 1, 0, 0, 0, 0, time.UTC)
	if err := s.Record([]Record{tomb(3, t0), pass(3, far)}, trusted, nowT); err == nil {
		t.Fatal("year 9999 accepted")
	}
	if got, _ := s.Lookup([][32]byte{key(3)}); len(got) != 0 {
		t.Fatal("rejected batch was partly written")
	}
}

func TestMergeRestampsDelta(t *testing.T) {
	far := time.Date(9999, 1, 1, 0, 0, 0, 0, time.UTC)
	// A forged year-9999 pass and a tombstone in the same delta: both are
	// stamped now, the tombstone wins the tie, the forger gains nothing.
	delta := deltaFile(t, pass(1, far), tomb(1, t0))
	out := filepath.Join(t.TempDir(), "out")
	if _, err := Merge(snapshotWith(t, pass(2, t0)), delta, out, trusted, nowT, nil); err != nil {
		t.Fatal(err)
	}
	got, _ := Open(out, Shared).Lookup([][32]byte{key(1), key(2)})
	if _, hit := got[key(1)]; hit {
		t.Fatal("forged far-future pass outranked a tombstone")
	}
	// Surviving delta records carry now, not the writer's time.
	out2 := filepath.Join(t.TempDir(), "out2")
	if _, err := Merge(snapshotWith(t), deltaFile(t, pass(5, far)), out2, trusted, nowT.Add(500*time.Millisecond), nil); err != nil {
		t.Fatal(err)
	}
	got, _ = Open(out2, Shared).Lookup([][32]byte{key(5)})
	if !got[key(5)].At.Equal(nowT) {
		t.Fatalf("At = %v, want %v", got[key(5)].At, nowT)
	}
}

func TestMergeFutureSnapshotRejected(t *testing.T) {
	far := time.Date(9999, 1, 1, 0, 0, 0, 0, time.UTC)
	dir := t.TempDir()
	data, _ := EncodeSegment([]Record{pass(1, far)})
	writeFile(t, dir, segmentName(1, "0123456789abcdef"), data)
	out := filepath.Join(t.TempDir(), "out")
	if _, err := Merge(dir, deltaFile(t, tomb(1, t0)), out, trusted, nowT, nil); err == nil ||
		!strings.Contains(err.Error(), "snapshot") {
		t.Fatalf("future snapshot record accepted: %v", err)
	}
	// Skew boundary: exactly MaxClockSkew ahead is fine, one second more is not.
	for _, c := range []struct {
		d  time.Duration
		ok bool
	}{{MaxClockSkew, true}, {MaxClockSkew + time.Second, false}} {
		snap := t.TempDir()
		data, _ := EncodeSegment([]Record{pass(1, nowT.Add(c.d))})
		writeFile(t, snap, segmentName(1, "0123456789abcdef"), data)
		_, err := Merge(snap, deltaFile(t), filepath.Join(t.TempDir(), "o"), trusted, nowT, nil)
		if (err == nil) != c.ok {
			t.Errorf("skew %v: %v", c.d, err)
		}
	}
	if _, err := Merge(snapshotWith(t), deltaFile(t), filepath.Join(t.TempDir(), "o"), trusted, time.Time{}, nil); err == nil {
		t.Error("zero now accepted")
	}
}

func TestEqualAtTombstoneWins(t *testing.T) {
	// Same file, both line orders.
	for _, recs := range [][]Record{{pass(1, t0), tomb(1, t0)}, {tomb(1, t0), pass(1, t0)}} {
		dir := t.TempDir()
		data, _ := EncodeSegment(recs)
		writeFile(t, dir, segmentName(1, "0123456789abcdef"), data)
		if got, err := Open(dir, Shared).Lookup([][32]byte{key(1)}); err != nil || len(got) != 0 {
			t.Errorf("same file, first %s: %v %v", recs[0].Kind, got, err)
		}
	}
	// Different segments, both sequence orders.
	for _, passSeq := range []uint64{1, 2} {
		dir := t.TempDir()
		pd, _ := EncodeSegment([]Record{pass(1, t0)})
		td, _ := EncodeSegment([]Record{tomb(1, t0)})
		writeFile(t, dir, segmentName(passSeq, "0123456789abcdef"), pd)
		writeFile(t, dir, segmentName(3-passSeq, "fedcba9876543210"), td)
		if got, _ := Open(dir, Shared).Lookup([][32]byte{key(1)}); len(got) != 0 {
			t.Errorf("pass in seq %d outranked an equal-At tombstone", passSeq)
		}
	}
	// Snapshot vs delta at the same (re-stamped) instant, both ways.
	for _, c := range []struct{ snapRec, deltaRec Record }{
		{tomb(1, nowT), pass(1, nowT)},
		{pass(1, nowT), tomb(1, nowT)},
	} {
		out := filepath.Join(t.TempDir(), "out")
		if _, err := Merge(snapshotWith(t, c.snapRec), deltaFile(t, c.deltaRec), out, trusted, nowT, nil); err != nil {
			t.Fatal(err)
		}
		if got, _ := Open(out, Shared).Lookup([][32]byte{key(1)}); len(got) != 0 {
			t.Errorf("%s over %s at equal time won as a pass", c.deltaRec.Kind, c.snapRec.Kind)
		}
	}
}

func TestRecordSyncFailure(t *testing.T) {
	s := Open(t.TempDir(), Shared)
	s.fs.syncDir = func(string) error { return errors.New("dir sync failed") }
	if err := s.Record([]Record{pass(1, t0)}, trusted, nowT); err == nil {
		t.Fatal("directory sync failure swallowed")
	}
}

func TestSyncDirReal(t *testing.T) {
	if err := syncDir(t.TempDir()); err != nil {
		t.Fatal(err)
	}
	if err := syncDir(filepath.Join(t.TempDir(), "missing")); err == nil {
		t.Log("platform does not sync directories")
	}
}

func partials(parent string) []string {
	ents, _ := os.ReadDir(parent)
	var out []string
	for _, e := range ents {
		if strings.Contains(e.Name(), ".partial-") {
			out = append(out, e.Name())
		}
	}
	return out
}

func TestMergeCleansPartialOnFailure(t *testing.T) {
	snap := snapshotWith(t, pass(1, t0))
	delta := deltaFile(t, pass(2, t0))
	boom := errors.New("boom")
	modes := map[string]fsOps{
		"create": {create: func(string) (tmpFile, error) { return nil, boom }},
		"rename": {create: realFS.create, syncDir: realFS.syncDir,
			rename: func(a, b string) error {
				if strings.Contains(filepath.Base(a), ".partial-") {
					return boom // the final directory rename
				}
				return os.Rename(a, b)
			}},
		"sync": {create: realFS.create, rename: os.Rename,
			syncDir: func(d string) error {
				if strings.Contains(filepath.Base(d), ".partial-") {
					return nil
				}
				return boom // syncing the parent after the directory rename
			}},
	}
	for name, ops := range modes {
		parent := t.TempDir()
		out := filepath.Join(parent, "out")
		if _, err := merge(ops, snap, delta, out, trusted, nowT, nil); err == nil {
			t.Errorf("%s: no error", name)
		}
		if p := partials(parent); len(p) != 0 {
			t.Errorf("%s: partial directory left: %v", name, p)
		}
		if name == "create" || name == "rename" {
			if _, err := os.Stat(out); err == nil {
				t.Errorf("%s: output exists after failure", name)
			}
		}
	}
	// Success leaves no partial directory either.
	parent := t.TempDir()
	if _, err := Merge(snap, delta, filepath.Join(parent, "out"), trusted, nowT, nil); err != nil {
		t.Fatal(err)
	}
	if p := partials(parent); len(p) != 0 {
		t.Errorf("partial left after success: %v", p)
	}
}

func TestMergeUncreatablePartial(t *testing.T) {
	file := writeFile(t, t.TempDir(), "f", nil)
	out := filepath.Join(file, "x", "out") // parent is a regular file
	if _, err := Merge(snapshotWith(t), deltaFile(t, pass(1, t0)), out, trusted, nowT, nil); err == nil {
		t.Error("uncreatable partial accepted")
	}
}

func TestMergeAliasedOutputRejected(t *testing.T) {
	base := t.TempDir()
	snap := filepath.Join(base, "snap")
	if err := os.Mkdir(snap, 0o755); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(base, "link")
	if err := os.Symlink(snap, link); err != nil {
		t.Skipf("symlinks unsupported here: %v", err)
	}
	delta := deltaFile(t, pass(1, t0))
	if _, err := Merge(snap, delta, link, trusted, nowT, nil); err == nil {
		t.Error("symlink alias of the snapshot accepted as output")
	}
	// A distinct path under the alias is a different directory.
	if _, err := Merge(snap, delta, filepath.Join(base, "new"), trusted, nowT, nil); err != nil {
		t.Errorf("distinct path rejected: %v", err)
	}
}

func TestResolvePath(t *testing.T) {
	d := t.TempDir()
	got, err := resolvePath(filepath.Join(d, "a", "b"))
	if err != nil {
		t.Fatal(err)
	}
	real, _ := filepath.EvalSymlinks(d)
	if got != filepath.Join(real, "a", "b") {
		t.Fatalf("got %s", got)
	}
}

func TestLoadDirCumulativeCaps(t *testing.T) {
	dir := t.TempDir()
	data, _ := EncodeSegment([]Record{pass(1, t0), pass(2, t0)})
	for i := 1; i <= 3; i++ {
		writeFile(t, dir, segmentName(uint64(i), "0123456789abcdef"), data)
	}
	if es, err := loadDirLimits(dir, int64(3*len(data)), 6); err != nil || len(es) != 6 {
		t.Fatalf("at limits: %v", err)
	}
	if _, err := loadDirLimits(dir, int64(3*len(data))-1, 6); err == nil || !strings.Contains(err.Error(), "budget") {
		t.Errorf("byte cap: %v", err)
	}
	if _, err := loadDirLimits(dir, 1<<20, 5); err == nil || !strings.Contains(err.Error(), "records") {
		t.Errorf("record cap: %v", err)
	}
}
