//go:build unix

package record

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// Tests of the two void tokens `collect` adds, `survivor` (run-void, M3) and
// `state-preloaded` (shard-void, M4), and of the marker RunStep leaves for the
// second. Nothing here sleeps: the survivor hook is a function and the wait is
// off or on a fake clock.

func hasVoid(sh *recfmt.Shard, tok string) bool {
	for _, v := range sh.Header.Void {
		if v == tok {
			return true
		}
	}
	return false
}

func TestCollectCleanRunHasNeitherToken(t *testing.T) {
	f := newCfix(t)
	f.good()
	_, sh := f.collect()
	if len(sh.Header.Void) != 0 {
		t.Fatalf("void %v", sh.Header.Void)
	}
	requireKnown(t, sh, ca)
}

func TestCollectSurvivorVoidsTheRunAndEveryRecord(t *testing.T) {
	f := newCfix(t)
	f.good()
	var gotRoot, gotCfg string
	var gotQ SurvivorQuery
	o := f.opts()
	o.Survivors = func(q SurvivorQuery) ([]string, error) {
		gotRoot, gotCfg, gotQ = q.RepoRoot, q.Configs[0], q
		return []string{"pid 4242: S1: it is in another mount namespace that has the checkout mounted"}, nil
	}
	res, err := Collect(o)
	if err != nil {
		t.Fatal(err)
	}
	if gotRoot != f.p.root || gotCfg != filepath.Join(f.p.dir, "pre", "sandbox.json") {
		t.Fatalf("the scan was given %q and %q", gotRoot, gotCfg)
	}
	// The probe sandbox's configuration is named too, and the start-time bound is
	// the sealed prepared_at less two seconds.
	if len(gotQ.Configs) != 2 || gotQ.Configs[1] != filepath.Join(f.p.dir, "probe", "toolchains-sandbox.json") {
		t.Fatalf("configs %v", gotQ.Configs)
	}
	if want, _ := time.Parse(time.RFC3339, f.pre.Binding.PreparedAt); !gotQ.Since.Equal(want.Add(-2 * time.Second)) {
		t.Fatalf("since %v, prepared_at %q", gotQ.Since, f.pre.Binding.PreparedAt)
	}
	sh := f.decode()
	if !hasVoid(sh, "survivor") || hasVoid(sh, "state-preloaded") || hasVoid(sh, "tracked-changed") {
		t.Fatalf("void %v", sh.Header.Void)
	}
	for _, a := range []string{ca, cb, cz} {
		requireUnknown(t, sh, a, "survivor")
	}
	if len(res.Unknown) != 3 || !strings.Contains(strings.Join(res.PostProblems, "|"), "survivor: pid 4242") {
		t.Fatalf("unknown %v problems %v", res.Unknown, res.PostProblems)
	}
}

func TestCollectSurvivorScanErrorIsASurvivor(t *testing.T) {
	f := newCfix(t)
	f.good()
	o := f.opts()
	o.Survivors = func(SurvivorQuery) ([]string, error) { return nil, errors.New("no /proc") }
	if _, err := Collect(o); err != nil {
		t.Fatal(err)
	}
	sh := f.decode()
	if !hasVoid(sh, "survivor") {
		t.Fatalf("void %v", sh.Header.Void)
	}
	requireUnknown(t, sh, ca, "survivor")
}

func TestCollectSurvivorThatGoesAwayWithinTheWaitIsNotOne(t *testing.T) {
	f := newCfix(t)
	f.good()
	o := f.opts()
	calls, slept := 0, time.Duration(0)
	o.Survivors = func(SurvivorQuery) ([]string, error) {
		calls++
		if calls < 3 {
			return []string{"pid 7: S2: x"}, nil
		}
		return nil, nil
	}
	o.SurvivorWait = 10 * time.Second
	now := time.Unix(0, 0)
	o.Now = func() time.Time { return now }
	o.Sleep = func(d time.Duration) { slept += d; now = now.Add(d) }
	if _, err := Collect(o); err != nil {
		t.Fatal(err)
	}
	if calls != 3 || slept != 2*survivorPoll {
		t.Fatalf("calls %d slept %v", calls, slept)
	}
	sh := f.decode()
	if len(sh.Header.Void) != 0 {
		t.Fatalf("void %v", sh.Header.Void)
	}
	requireKnown(t, sh, ca)
}

func TestCollectSurvivorStillThereAfterTheWaitVoids(t *testing.T) {
	f := newCfix(t)
	f.good()
	o := f.opts()
	o.SurvivorWait = time.Second
	now := time.Unix(0, 0)
	o.Now = func() time.Time { return now }
	o.Sleep = func(d time.Duration) { now = now.Add(d) }
	o.Survivors = func(SurvivorQuery) ([]string, error) { return []string{"pid 7: S2: x"}, nil }
	if _, err := Collect(o); err != nil {
		t.Fatal(err)
	}
	if !hasVoid(f.decode(), "survivor") {
		t.Fatal("not void")
	}
}

func TestCollectScansBeforeItMarksTheDirectoryCollected(t *testing.T) {
	// The scan runs under the exclusive lock, after pre/ is verified and before
	// the tombstone: a refusal that follows (here a bad --root) leaves the
	// directory collectable and never reaches the scan with an unverified pre/.
	f := newCfix(t)
	f.good()
	o := f.opts()
	scanned := false
	o.Survivors = func(SurvivorQuery) ([]string, error) { scanned = true; return nil, nil }
	o.Root = filepath.Join(filepath.Dir(f.p.root), "elsewhere")
	if _, err := Collect(o); err == nil {
		t.Fatal("a wrong --root was accepted")
	}
	if scanned {
		t.Fatal("the scan ran for a directory whose --root was refused")
	}
}

func TestCollectStatePreloadedMarkerVoidsTheShardOnly(t *testing.T) {
	f := newCfix(t)
	f.good()
	marker := filepath.Join(f.p.dir, "traces", StatePreloadedMarker)
	if err := os.WriteFile(marker, []byte("/home/runner/.cargo/registry"), 0o600); err != nil {
		t.Fatal(err)
	}
	res, err := Collect(f.opts())
	if err != nil {
		t.Fatal(err)
	}
	sh := f.decode()
	if !hasVoid(sh, "state-preloaded") || hasVoid(sh, "survivor") {
		t.Fatalf("void %v", sh.Header.Void)
	}
	// The marker is a known entry of traces/: the serial order still holds, so
	// the records stay believed (the merge voids the SHARD's records, M4).
	for _, a := range []string{ca, cb, cz} {
		requireKnown(t, sh, a)
	}
	if !strings.Contains(strings.Join(res.PostProblems, "|"), "state-preloaded: a state root was not empty when the first step started: /home/runner/.cargo/registry") {
		t.Fatalf("problems %v", res.PostProblems)
	}
}

func TestCollectStatePreloadedMarkerOfAnyKindVoids(t *testing.T) {
	f := newCfix(t)
	f.good()
	// A directory, not a file: not what RunStep writes, and void all the same.
	if err := os.Mkdir(filepath.Join(f.p.dir, "traces", StatePreloadedMarker), 0o700); err != nil {
		t.Fatal(err)
	}
	if _, err := Collect(f.opts()); err != nil {
		t.Fatal(err)
	}
	if !hasVoid(f.decode(), "state-preloaded") {
		t.Fatal("not void")
	}
}

func TestCollectBothTokensTogether(t *testing.T) {
	f := newCfix(t)
	f.good()
	if err := os.WriteFile(filepath.Join(f.p.dir, "traces", StatePreloadedMarker), []byte("/s"), 0o600); err != nil {
		t.Fatal(err)
	}
	o := f.opts()
	o.Survivors = func(SurvivorQuery) ([]string, error) { return []string{"pid 1: S2: x"}, nil }
	if _, err := Collect(o); err != nil {
		t.Fatal(err)
	}
	sh := f.decode()
	if !hasVoid(sh, "survivor") || !hasVoid(sh, "state-preloaded") || len(sh.Header.Void) != 2 {
		t.Fatalf("void %v", sh.Header.Void)
	}
	requireUnknown(t, sh, ca, "survivor")
}

// ---------------------------------------------------------------------------
// RunStep's seq-0 check.

// stateStepFix is a step fixture whose configuration has one state root,
// $HOME/.state.
func stateStepFix(t *testing.T) (*stepFix, string) {
	t.Helper()
	f := newStepFixRoots(t, []string{`{"path": "$HOME/.state", "kind": "state", "ecosystem": "x"}`}, planSteps, nil, nil, false)
	if len(f.pre.Config.StateRoots) != 1 {
		t.Fatalf("state roots %v", f.pre.Config.StateRoots)
	}
	return f, f.pre.Config.StateRoots[0]
}

func markerPath(f *stepFix) string {
	return filepath.Join(f.p.dir, "traces", StatePreloadedMarker)
}

func TestStepSeqZeroWritesTheMarkerForAPreloadedStateRoot(t *testing.T) {
	f, state := stateStepFix(t)
	writeBytes(t, filepath.Join(state, "cached"), []byte("x"), 0o644)
	res := f.must(actA, 0, 1, "exit 0")
	// The step still ran, and its sidecar is the first number of the order.
	if res.ExitCode != 0 || res.Sidecar.Seq != 0 || f.runs() != 1 {
		t.Fatalf("%+v runs %d", res, f.runs())
	}
	data, err := os.ReadFile(markerPath(f))
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != state {
		t.Fatalf("marker %q, want %q", data, state)
	}
}

func TestStepSeqZeroLeavesNoMarkerForAnEmptyOrAbsentStateRoot(t *testing.T) {
	f, state := stateStepFix(t)
	if err := os.MkdirAll(state, 0o755); err != nil { // present and empty
		t.Fatal(err)
	}
	f.must(actA, 0, 1, "exit 0")
	if _, err := os.Lstat(markerPath(f)); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("a marker for an empty state root: %v", err)
	}
}

func TestStepMarkerIsOnlyWrittenAtSeqZero(t *testing.T) {
	f, state := stateStepFix(t)
	f.must(actA, 0, 1, "exit 0") // seq 0: the roots are empty then
	writeBytes(t, filepath.Join(state, "cached"), []byte("x"), 0o644)
	f.must(actB, 0, 2, "exit 0") // seq 1
	f.must(actB, 1, 2, "exit 0") // seq 2
	if _, err := os.Lstat(markerPath(f)); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("a marker written after seq 0: %v", err)
	}
}

func TestStepSeqZeroStateRootThatIsNotADirectoryIsPreloaded(t *testing.T) {
	f, state := stateStepFix(t)
	if err := os.Symlink("/tmp", state); err != nil { // a root that is not a real directory
		t.Fatal(err)
	}
	f.must(actA, 0, 1, "exit 0")
	if _, err := os.Lstat(markerPath(f)); err != nil {
		t.Fatalf("no marker for a state root that is a symlink: %v", err)
	}
}

func TestStepMarkerThatCannotBeWrittenRefusesTheStep(t *testing.T) {
	f, state := stateStepFix(t)
	writeBytes(t, filepath.Join(state, "cached"), []byte("x"), 0o644)
	// traces/ cannot take a new file: the step that cannot say the shard is void
	// does not run, and the number it took is a gap collect sees.
	traces := filepath.Join(f.p.dir, "traces")
	// (The action's own directory exists already, so only the marker needs a
	// new entry in traces/.)
	if err := os.Mkdir(StepDir(f.p.dir, actA), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(traces, ".serial"), nil, 0o600); err != nil { // the lock file, made by the first step
		t.Fatal(err)
	}
	if err := os.Chmod(traces, 0o500); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.Chmod(traces, 0o700) })
	if os.Geteuid() == 0 {
		t.Skip("root writes into a read-only directory")
	}
	before := f.runs()
	if _, err := f.run(actA, 0, 1, "exit 0"); err == nil || !strings.Contains(err.Error(), StatePreloadedMarker) {
		t.Fatalf("a step ran whose marker could not be left: %v", err)
	}
	if f.runs() != before {
		t.Fatal("the tracer started")
	}
}
