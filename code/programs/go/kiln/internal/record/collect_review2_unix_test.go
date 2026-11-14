//go:build unix

package record

// Tests added by the second review of `collect` (KILN04 stage 4 PR 2b-2b-ii).

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// A step that cannot tell whether the directory was collected does not run: only
// "the tombstone is not there" lets it go on.
func TestStepTombstoneCheckFailsClosed(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root can search any directory")
	}
	f := newCfix(t)
	f.good()
	traces := filepath.Join(f.p.dir, "traces")
	if err := os.Chmod(traces, 0); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.Chmod(traces, 0o700) })
	env := append(append([]string{}, f.pre.Tools.BuildToolEnv...), EnvAction+"="+ca, EnvStep+"=0", EnvStepCount+"=1")
	_, err := RunStep(StepOptions{Dir: f.p.dir, Argv: []string{"sh", "-c", "echo a"}, Environ: env, Cwd: f.pre.Config.PackageDir(ca), KilnExe: f.p.kiln})
	if err == nil || !strings.Contains(err.Error(), "cannot be checked") {
		t.Fatalf("%v", err)
	}
}

// --results needs a canonical parent: a symlinked one, here pointing INTO the
// checkout, is refused even though its spelling is outside it.
func TestCollectRefusesAResultsParentThatIsALink(t *testing.T) {
	f := newCfix(t)
	f.good()
	inside := filepath.Join(f.p.root, "code")
	link := filepath.Join(f.p.base, "rlink")
	if err := os.Symlink(inside, link); err != nil {
		t.Skip(err)
	}
	o := f.opts()
	o.Results = filepath.Join(link, "results.json")
	if _, err := Collect(o); err == nil || !strings.Contains(err.Error(), "the parent of --results") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Lstat(filepath.Join(f.p.dir, "traces", CollectedMarker)); err == nil {
		t.Fatal("a refused argument used up the collection")
	}
	// A canonical parent outside the checkout is fine.
	f.collect()
}

// writeAtomic that cannot make its link durable takes the link back, so "failed"
// always means "nothing written".
func TestWriteAtomicTakesTheLinkBackWhenTheSyncFails(t *testing.T) {
	old := syncDirFn
	syncDirFn = func(string) error { return errors.New("input/output error") }
	t.Cleanup(func() { syncDirFn = old })
	d := canonTemp(t)
	final := filepath.Join(d, "records.jsonl")
	if err := writeAtomic(final, []byte("data")); err == nil || !strings.Contains(err.Error(), "input/output") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Lstat(final); err == nil {
		t.Fatal("a failed write left the file")
	}
	if ents, _ := os.ReadDir(d); len(ents) != 0 {
		t.Fatalf("left behind: %v", ents)
	}
}

func TestCollectOutIsNotLeftWhenItsWriteFails(t *testing.T) {
	f := newCfix(t)
	f.good()
	old := syncDirFn
	syncDirFn = func(string) error { return errors.New("no space left on device") }
	t.Cleanup(func() { syncDirFn = old })
	if _, err := Collect(f.opts()); err == nil || !strings.Contains(err.Error(), "--out") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Lstat(f.out); err == nil {
		t.Fatal("--out exists after a failed write")
	}
	// The directory is burned (spec 4.8): the next collect is refused. A missing
	// shard is run-void in the merge (B2), so this fails closed.
	syncDirFn = old
	if _, err := Collect(f.opts()); err == nil || !strings.Contains(err.Error(), "collected once") {
		t.Fatalf("%v", err)
	}
}

// The classification child that cannot write record.json durably reports an
// error (so the supervisor leaves record.failed) and leaves no record.json.
func TestClassifyChildLeavesNoRecordWhenTheSyncFails(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	f.must(actZ, 0, 1, line)
	if err := os.Remove(RecordFile(f.p.dir, actZ)); err != nil {
		t.Fatal(err)
	}
	old := syncDirFn
	syncDirFn = func(string) error { return errors.New("input/output error") }
	t.Cleanup(func() { syncDirFn = old })
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, ChildHooks{}); err == nil || !strings.Contains(err.Error(), "record.json") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Lstat(RecordFile(f.p.dir, actZ)); err == nil {
		t.Fatal("record.json was left after a failed write")
	}
}

// scanAction keeps no more than maxScanBad reasons however many odd entries there are.
func TestScanActionCapsItsReasons(t *testing.T) {
	d := canonTemp(t)
	for i := 0; i < 300; i++ {
		os.WriteFile(filepath.Join(d, fmt.Sprintf("odd%03d.txt", i)), nil, 0o600)
	}
	s := scanAction(d)
	if len(s.bad) != maxScanBad {
		t.Fatalf("%d reasons", len(s.bad))
	}
	if last := s.bad[len(s.bad)-1]; !strings.Contains(last, "... and 237 more") {
		t.Fatalf("%q", last)
	}
	// Fewer than the cap: all kept, nothing summarised.
	d2 := canonTemp(t)
	for i := 0; i < 5; i++ {
		os.WriteFile(filepath.Join(d2, fmt.Sprintf("odd%d.txt", i)), nil, 0o600)
	}
	if s := scanAction(d2); len(s.bad) != 5 || strings.Contains(strings.Join(s.bad, ""), "more") {
		t.Fatalf("%v", s.bad)
	}
}

// A classified record with tens of thousands of path strings is still decoded
// strictly (the walk skips only lists that have no keys to check).
func TestDecodeClassifiedLargeStringListsStayStrict(t *testing.T) {
	raw := rawFor("code/x")
	for i := 0; i < 60000; i++ {
		raw.Writes = append(raw.Writes, fmt.Sprintf("code/x/out/%06d", i))
	}
	c := ClassifiedRecord{
		Schema: ClassifiedSchema, Action: "code/x", BindingID: strings.Repeat("a", 64), SealedSHA256: strings.Repeat("b", 64),
		StepsRun: 1, StepNetNS: true, ParseOK: true, ParseReasons: []string{}, WritesComplete: true, Record: raw,
	}
	data, err := EncodeClassified(c)
	if err != nil {
		t.Fatal(err)
	}
	got, err := DecodeClassified(data)
	if err != nil || len(got.Record.Writes) != 60001 {
		t.Fatalf("%v", err)
	}
	// A duplicate key beside the big list is still found.
	bad := strings.Replace(string(data), `"steps_run":1`, `"steps_run":1,"steps_run":1`, 1)
	if _, err := DecodeClassified([]byte(bad)); err == nil {
		t.Fatal("a duplicate key beside a large list was accepted")
	}
}
