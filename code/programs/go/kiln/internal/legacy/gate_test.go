package legacy

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

// agree builds the plan and a Traced from the same options and requires the
// Traced's gate to equal the plan's for EVERY package, then returns the gates.
func (e *lkEnv) agree() map[string]string {
	e.t.Helper()
	plan, err := BuildPlan(e.g, e.opts("linux"))
	if err != nil {
		e.t.Fatalf("BuildPlan: %v", err)
	}
	tr := e.traced()
	out := map[string]string{}
	for i, p := range e.g.Packages {
		got, err := tr.Gate(p.RelPath)
		if err != nil {
			e.t.Fatalf("Gate(%s): %v", p.RelPath, err)
		}
		if want := plan.Packages[i].gate; got != want {
			e.t.Errorf("%s: Traced.Gate %q, plan %q", p.Name, got, want)
		}
		out[p.Name] = got
	}
	return out
}

func (e *lkEnv) pkg(name string) *Package {
	for i := range e.g.Packages {
		if e.g.Packages[i].Name == name {
			return &e.g.Packages[i]
		}
	}
	e.t.Fatalf("no package %s", name)
	return nil
}

func TestGateMatchesPlan(t *testing.T) {
	t.Run("nothing gates", func(t *testing.T) {
		for n, g := range newLk(t).agree() {
			if g != "" {
				t.Errorf("%s: %q", n, g)
			}
		}
	})
	t.Run("never", func(t *testing.T) {
		e := newLk(t)
		e.f.write(NeverPath, `{"packages": ["x/z"]}`)
		if g := e.agree()["x/z"]; g != "never:x/z" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("never beats a missing BUILD", func(t *testing.T) {
		e := newLk(t)
		e.f.write(NeverPath, `{"packages": ["x/z"]}`)
		e.pkg("x/z").BuildFiles["linux"] = nil
		if g := e.agree()["x/z"]; g != "never:x/z" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("missing linux BUILD", func(t *testing.T) {
		e := newLk(t)
		e.pkg("x/z").BuildFiles["linux"] = nil
		if g := e.agree()["x/z"]; g != "no-build-file:linux" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("unknown toolchain", func(t *testing.T) {
		e := newLk(t)
		e.pkg("x/z").Language = "nolang"
		if g := e.agree()["x/z"]; g != "unknown-toolchain:nolang" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("unknown repo inputs", func(t *testing.T) {
		e := newLk(t)
		e.pkg("x/z").RepoInputsUnknown = true
		if g := e.agree()["x/z"]; g != "unknown-repo-inputs:x/z" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("symlinked BUILD", func(t *testing.T) {
		if runtime.GOOS == "windows" {
			t.Skip("symlinks")
		}
		e := newLk(t)
		build := filepath.Join(e.f.dir, "code/packages/x/z/BUILD")
		if err := os.Remove(build); err != nil {
			t.Fatal(err)
		}
		if err := os.Symlink("src.txt", build); err != nil {
			t.Fatal(err)
		}
		if g := e.agree()["x/z"]; g != "symlink:BUILD" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("symlink reason from the scan", func(t *testing.T) {
		if runtime.GOOS == "windows" {
			t.Skip("symlinks")
		}
		e := newLk(t)
		// A repo input that is a link out of the package's region is a KILN03
		// symlink reason, which gates like any hard reason.
		link := filepath.Join(e.f.dir, "code/shared/y.json")
		if err := os.Remove(link); err != nil {
			t.Fatal(err)
		}
		if err := os.Symlink("../packages/x/z/src.txt", link); err != nil {
			t.Fatal(err)
		}
		e.agree() // whatever the scan decides, plan and recorder agree
	})
	t.Run("probe mismatch", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(false, "xtool 9.9.9")
		if g := e.agree()["x/z"]; g != "probe-mismatch:xtool" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("probe agreement", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(false, "xtool 1.2.3")
		if g := e.agree()["x/z"]; g != "" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("probe refused", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(true, "xtool 1.2.3")
		if g := e.agree()["x/z"]; g != "probe-refused:xtool" {
			t.Fatalf("%q", g)
		}
	})
	t.Run("probe timeout", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(false, "xtool 1.2.3")
		e.mod = func(o *Options) {
			o.Probe = &ProbeOptions{Root: e.t.TempDir(), Total: time.Nanosecond,
				Run: func(ProbeRun) ([]byte, error) { time.Sleep(2 * time.Millisecond); return []byte("xtool 1.2.3"), nil }}
		}
		e.agree()
	})
}

func TestGatePriorityAndTieBreak(t *testing.T) {
	pl := &planner{}
	// never beats every other hard reason, which beat every probe.
	f := gateFacts{
		symlinks: []string{"symlink:z"}, unknownInputs: []string{"b"}, unknownTools: []string{"zz", "aa"},
		never: true, recipeWhy: "no-build-file:linux", probe: []string{"probe-mismatch:a"},
	}
	if g := pl.gateFrom("p", f); g != "never:p" {
		t.Errorf("%q", g)
	}
	f.never = false
	if g := pl.gateFrom("p", f); g != "no-build-file:linux" {
		t.Errorf("hard reasons: smallest string first: %q", g)
	}
	f.recipeWhy, f.symlinks, f.unknownInputs = "", nil, nil
	if g := pl.gateFrom("p", f); g != "unknown-toolchain:aa" {
		t.Errorf("%q", g)
	}
	f.unknownTools = nil
	f.probe = []string{"probe-timeout:b", "probe-mismatch:b", "probe-refused:a"}
	if g := pl.gateFrom("p", f); g != "probe-mismatch:b" {
		t.Errorf("probes: smallest string first: %q", g)
	}
	if g := pl.gateFrom("p", gateFacts{}); g != "" {
		t.Errorf("%q", g)
	}
}

func TestTracedGateErrorsAndProbes(t *testing.T) {
	e := newLk(t)
	tr := e.traced()
	if _, err := tr.Gate("code/packages/x/ghost"); err == nil {
		t.Error("Gate of an unknown package must fail")
	}
	if p, err := tr.Probes(); err != nil || len(p) != 0 {
		t.Errorf("no --probe: %v %v", p, err)
	}

	// Probes are the plan's: run once, however many Gate calls follow.
	e = newLk(t)
	e.probeEnv(false, "xtool 9.9.9")
	runs := 0
	inner := e.mod
	e.mod = func(o *Options) {
		inner(o)
		run := o.Probe.Run
		o.Probe.Run = func(r ProbeRun) ([]byte, error) { runs++; return run(r) }
	}
	tr = e.traced()
	for _, p := range e.g.Packages {
		if _, err := tr.Gate(p.RelPath); err != nil {
			t.Fatal(err)
		}
	}
	probes, err := tr.Probes()
	found := false
	for _, p := range probes {
		found = found || p.Tool == "xtool" && p.Status == ProbeMismatch
	}
	if err != nil || !found {
		t.Fatalf("%v %v", probes, err)
	}
	if runs == 0 || runs > 2 {
		t.Errorf("the probes ran %d times", runs)
	}

	// A broken boundary file fails the preparation, for every call.
	e = newLk(t)
	e.f.write(BoundaryPath, "not json")
	tr = e.traced()
	if _, err := tr.Gate("code/packages/x/z"); err == nil {
		t.Error("a bad boundary file must fail Gate")
	}
	if _, err := tr.Probes(); err == nil {
		t.Error("a bad boundary file must fail Probes")
	}
}

func TestProbeOptionsTimeout(t *testing.T) {
	e := newLk(t)
	e.probeEnv(false, "xtool 1.2.3")
	var got []time.Duration
	e.mod = func(o *Options) {
		o.Probe = &ProbeOptions{Root: e.t.TempDir(), Timeout: 7 * time.Second,
			Run: func(r ProbeRun) ([]byte, error) { got = append(got, r.Timeout); return []byte("xtool 1.2.3"), nil }}
	}
	if _, err := BuildPlan(e.g, e.opts("linux")); err != nil {
		t.Fatal(err)
	}
	if len(got) == 0 {
		t.Skip("no probe ran")
	}
	for _, d := range got {
		if d != 7*time.Second {
			t.Errorf("timeout %v", d)
		}
	}
}
