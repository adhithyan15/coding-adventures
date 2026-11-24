//go:build unix

package record

// The census-only --allow-preloaded-state flag (Options.AllowPreloadedState).
//
// The census runs on a runner with no network, so cargo needs its registry
// already on disk, which `prepare` would otherwise refuse (`state-preloaded`).
// The flag skips ONLY that fail-fast refusal. The protection is the second
// check: RunStep's seq-0 marker and collect's `state-preloaded` shard-void.
// These tests pin both halves: the refusal stays without the flag, and with
// it the shard is still void at collect.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

const preloadedRoot = `{"path": "$HOME/.state", "kind": "state", "ecosystem": "x"}`

func TestPrepareStillRefusesAPreloadedRootWithoutTheFlag(t *testing.T) {
	p := newPrep(t)
	p.roots = []string{preloadedRoot}
	p.commit()
	writeBytes(t, filepath.Join(p.home, ".state", "cached"), []byte("x"), 0o644)
	if _, err := p.prepare(); err == nil || !strings.Contains(err.Error(), "unscrubbed:state-preloaded") {
		t.Fatalf("want a state-preloaded refusal, got %v", err)
	}
}

func TestPrepareWithTheFlagAcceptsAPreloadedRootAndTheShardIsVoid(t *testing.T) {
	p := newPrep(t)
	p.roots = []string{preloadedRoot}
	p.allowPre = true
	p.steps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
		`{"package":"code/packages/x/a","steps":["echo a"]},` +
		`{"package":"code/packages/x/b","steps":["echo b1","echo b2"]},` +
		`{"package":"code/packages/x/z","steps":["echo z"]}]}`
	p.commit()
	state := filepath.Join(p.home, ".state")
	writeBytes(t, filepath.Join(state, "cached"), []byte("x"), 0o644)

	f := finishCfix(t, p, "", singlePlan) // Prepare must succeed
	if len(f.pre.Config.StateRoots) != 1 || f.pre.Config.StateRoots[0] != state {
		t.Fatalf("state roots %v", f.pre.Config.StateRoots)
	}

	// What RunStep does at seq 0, with the very function it calls: the root is
	// not empty, so the marker is left.
	if err := markStatePreloaded(f.p.dir, f.pre.Config.StateRoots); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Lstat(filepath.Join(f.p.dir, "traces", StatePreloadedMarker)); err != nil {
		t.Fatalf("no marker for a preloaded root: %v", err)
	}
	f.good()
	_, sh := f.collect()
	if !hasVoid(sh, "state-preloaded") {
		t.Fatalf("void %v, want state-preloaded", sh.Header.Void)
	}
}

func TestRunStepAgainstAPreloadedRootPreparedWithTheFlagLeavesTheMarker(t *testing.T) {
	f := newStepFixRootsPre(t)
	state := f.pre.Config.StateRoots[0]
	res := f.must(actA, 0, 1, "exit 0")
	if res.ExitCode != 0 {
		t.Fatalf("%+v", res)
	}
	data, err := os.ReadFile(markerPath(f))
	if err != nil || string(data) != state {
		t.Fatalf("marker %q err %v, want %q", data, err, state)
	}
}

func TestProbesLeftNothingWithTheFlagSkipsOnlyTheStateHalf(t *testing.T) {
	base := t.TempDir()
	tmp, state := filepath.Join(base, "tmp"), filepath.Join(base, "state")
	os.Mkdir(tmp, 0o755)
	os.Mkdir(state, 0o755)
	os.WriteFile(filepath.Join(state, "f"), nil, 0o644)
	cfg := &sandbox.Config{Tmpdir: tmp, StateRoots: []string{state}}
	if err := probesLeftNothing(cfg, true); err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(tmp, "f"), nil, 0o644)
	if err := probesLeftNothing(cfg, true); err == nil || !strings.Contains(err.Error(), "tmpdir-written") {
		t.Fatalf("the tmpdir check must still hold: %v", err)
	}
}
