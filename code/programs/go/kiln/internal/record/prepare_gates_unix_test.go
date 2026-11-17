//go:build unix

package record

// `prepare` and the gates (stage 4 PR 2b-2c-3, decision P7): every action file
// says why its package must not be recorded as a measurement, and the sealed
// probes.json lists the toolchain probes' results. The probes run through the
// fake Options.ProbeRun; the sandboxed runner is tested on its own
// (probe_toolchain_unix_test.go) and end to end on Linux (TestSandboxRecordGates).

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// gateOf reads the sealed action file of rel.
func gateOf(t *testing.T, res *PrepareResult, rel string) Action {
	t.Helper()
	a, err := DecodeAction(readPre(t, res, ActionFile(rel)))
	if err != nil {
		t.Fatal(err)
	}
	return a
}

func TestPrepareNothingGatesByDefault(t *testing.T) {
	p := newPrep(t)
	p.withProbeBin("xtool")
	p.commit()
	res := p.mustPrepare()
	for _, rel := range []string{"code/packages/x/a", "code/packages/x/z"} {
		if a := gateOf(t, res, rel); a.Gate != "" || a.Schema != "kiln-record-action/3" {
			t.Errorf("%s: %+v", rel, a)
		}
	}
	rep, err := DecodeProbes(readPre(t, res, ProbesFile))
	if err != nil {
		t.Fatal(err)
	}
	// cc is pinned to the image (nothing to compare); xtool printed its pin.
	want := []ToolchainProbe{{Tool: "cc", Status: legacy.ProbeOK}, {Tool: "xtool", Status: legacy.ProbeOK}}
	if len(rep.Toolchains) != 2 || rep.Toolchains[0] != want[0] || rep.Toolchains[1] != want[1] {
		t.Fatalf("%+v", rep.Toolchains)
	}
	// The fake was asked once, under the name the PATH gave, with the verified
	// path of the binary and a timeout and a total as decision D8 says.
	if len(p.probeRuns) != 2 {
		t.Fatalf("%d probe runs", len(p.probeRuns))
	}
	r := p.probeRuns[1]
	if r.Argv0 != "xtool" || r.Timeout != ProbeTimeout || len(r.Args) != 1 || r.Args[0] != "--version" || filepath.Base(r.Path) != "xtool" {
		t.Fatalf("%+v", r)
	}
	if _, err := VerifyPre(p.dir); err != nil {
		t.Fatal(err)
	}
}

func TestPrepareGatesFollowThePlan(t *testing.T) {
	t.Run("a probe that disagrees gates every package of the language", func(t *testing.T) {
		p := newPrep(t)
		p.withProbeBin("xtool")
		p.probeRunFn = func(legacy.ProbeRun) ([]byte, error) { return []byte("xtool 9.9.9\n"), nil }
		p.commit()
		res := p.mustPrepare()
		for _, rel := range []string{"code/packages/x/a", "code/packages/x/z"} {
			if a := gateOf(t, res, rel); a.Gate != "probe-mismatch:xtool" {
				t.Errorf("%s: gate %q", rel, a.Gate)
			}
		}
		rep, _ := DecodeProbes(readPre(t, res, ProbesFile))
		if len(rep.Toolchains) != 2 || rep.Toolchains[1].Status != legacy.ProbeMismatch || !strings.Contains(rep.Toolchains[1].Detail, "9.9.9") {
			t.Fatalf("%+v", rep.Toolchains)
		}
	})
	t.Run("a probe that fails or times out is a mismatch too", func(t *testing.T) {
		p := newPrep(t)
		p.withProbeBin("xtool")
		p.probeRunFn = func(legacy.ProbeRun) ([]byte, error) { return []byte("xtool 1.2.3"), errors.New("the probe exited 3") }
		p.commit()
		if a := gateOf(t, p.mustPrepare(), "code/packages/x/a"); a.Gate != "probe-mismatch:xtool" {
			t.Fatalf("%q", a.Gate)
		}
	})
	t.Run("the never list", func(t *testing.T) {
		p := newPrep(t)
		p.files["code/kiln/legacy/never.json"] = `{"packages": ["x/z"]}`
		p.commit()
		res := p.mustPrepare()
		if a := gateOf(t, res, "code/packages/x/z"); a.Gate != "never:x/z" || a.Weak == "" {
			t.Fatalf("%+v", a)
		}
		if a := gateOf(t, res, "code/packages/x/a"); a.Gate != "" {
			t.Fatalf("%+v", a)
		}
	})
	t.Run("an unknown toolchain is a refusal and a gate", func(t *testing.T) {
		p := newPrep(t)
		g := string(p.graphJSON())
		i := strings.LastIndex(g, `"language":"x"`)
		p.graph = g[:i] + `"language":"nolang"` + g[i+len(`"language":"x"`):]
		p.commit()
		res := p.mustPrepare()
		a := gateOf(t, res, "code/packages/x/z")
		if a.Gate != "unknown-toolchain:nolang" || a.Refusal != "unknown-toolchain:nolang" || a.Weak != "" {
			t.Fatalf("%+v", a)
		}
		if a := gateOf(t, res, "code/packages/x/a"); a.Gate != "" {
			t.Fatalf("%+v", a)
		}
	})
	t.Run("a missing Linux BUILD is a gate", func(t *testing.T) {
		p := newPrep(t)
		g := string(p.graphJSON())
		i := strings.LastIndex(g, `"linux":"BUILD"`)
		if i < 0 {
			t.Skip("graph shape")
		}
		p.graph = g[:i] + `"linux":null` + g[i+len(`"linux":"BUILD"`):]
		p.commit()
		if a := gateOf(t, p.mustPrepare(), "code/packages/x/z"); a.Gate != "no-build-file:linux" {
			t.Fatalf("%q", a.Gate)
		}
	})
}

func TestPrepareClipsALongGate(t *testing.T) {
	p := newPrep(t)
	// A package name longer than a record's reason field: the gate is clipped
	// before it is sealed, and still non-empty.
	long := "x/" + strings.Repeat("n", 300)
	p.files["code/kiln/legacy/never.json"] = `{"packages": ["` + long + `"]}`
	g := string(p.graphJSON())
	p.graph = strings.Replace(g, `"name":"x/z"`, `"name":"`+long+`"`, 1)
	p.steps = ""
	p.commit()
	res, err := p.prepare()
	if err != nil {
		t.Skipf("the fixture's plan does not accept a long name: %v", err)
	}
	a := gateOf(t, res, "code/packages/x/z")
	if a.Gate == "" || len(a.Gate) > MaxGateBytes || !strings.HasPrefix(a.Gate, "never:x/nnn") {
		t.Fatalf("%q", a.Gate)
	}
}

func TestPrepareFailsWhenTheProbesCannotRun(t *testing.T) {
	t.Run("the real prober needs the sandbox: unarmed use is refused", func(t *testing.T) {
		// A runner left unset selects the SANDBOXED prober, never the host: with
		// the structure probe faked there is nothing for it to run, but Arm must
		// still make its directories.
		p := newPrep(t)
		p.withProbeBin("xtool")
		p.commit()
		o := p.options()
		o.ProbeRun = nil
		o.Structure = func(in ProbeInput) (ProbeReport, error) { return fakeProbeReport(in), nil }
		p.t.Cleanup(p.writable)
		_, err := Prepare(o)
		// kiln is a fake file, so the sandbox cannot start: a recorder error, not
		// a mismatch, and nothing is sealed.
		if err == nil || !strings.Contains(err.Error(), "toolchain probes") {
			t.Fatalf("%v", err)
		}
		if _, statErr := os.Lstat(filepath.Join(p.dir, "pre", "SEALED")); statErr == nil {
			t.Fatal("a failed probe left a sealed record")
		}
		if _, statErr := os.Lstat(filepath.Join(p.dir, "probe", "toolchains-sandbox.json")); statErr != nil {
			t.Errorf("the probes' configuration was not written: %v", statErr)
		}
	})
}

func TestPrepareFailsWhenAProbeLeavesSomethingInTheRealTmpdir(t *testing.T) {
	// The fake probe runs in-process, so it can reach the steps' real tmpdir,
	// as a probe that escaped its own TMPDIR would. prepare re-checks tmpdir
	// and every state root after the toolchain probes.
	p := newPrep(t)
	p.commit()
	p.probeRunFn = func(r legacy.ProbeRun) ([]byte, error) {
		if err := os.WriteFile(filepath.Join(p.tmpdir, "planted"), []byte("x"), 0o600); err != nil {
			t.Fatal(err)
		}
		return []byte(r.Argv0 + " version 1.2.3\n"), nil
	}
	_, err := p.prepare()
	if err == nil || !strings.Contains(err.Error(), "unscrubbed:tmpdir-written") {
		t.Fatalf("%v", err)
	}
	if _, statErr := os.Lstat(filepath.Join(p.dir, "pre", "SEALED")); statErr == nil {
		t.Fatal("a failed prepare left a sealed record")
	}
}

func TestPrepareFailsWhenAProbeLeavesSomethingInAStateRoot(t *testing.T) {
	p := newPrep(t)
	state := filepath.Join(p.base, "state")
	if err := os.Mkdir(state, 0o755); err != nil {
		t.Fatal(err)
	}
	p.roots = []string{`{"path": "` + state + `", "kind": "state", "ecosystem": "fixture"}`}
	p.commit()
	p.probeRunFn = func(r legacy.ProbeRun) ([]byte, error) {
		if err := os.WriteFile(filepath.Join(state, "planted"), []byte("x"), 0o600); err != nil {
			t.Fatal(err)
		}
		return []byte(r.Argv0 + " version 1.2.3\n"), nil
	}
	_, err := p.prepare()
	if err == nil || !strings.Contains(err.Error(), "unscrubbed:state-preloaded") {
		t.Fatalf("%v", err)
	}
}

func TestPrepareTreatsAHookErrorAsAMismatch(t *testing.T) {
	// The injected runner cannot report an exit status, so this goes through the
	// prober's own bookkeeping: see TestProberOneRefusalIsAMismatchButAllRefusedIsBroken.
	// Here: a recorder error from the runner reaches prepare through Err() only
	// for the sandboxed prober, so the hook (which has no Err) is a mismatch.
	p := newPrep(t)
	p.commit()
	p.probeRunFn = func(legacy.ProbeRun) ([]byte, error) {
		return nil, errors.New("the sandbox refused the probe (exit 125)")
	}
	res, err := p.prepare()
	if err != nil {
		t.Fatalf("a hook error is a mismatch: %v", err)
	}
	if a := gateOf(t, res, "code/packages/x/a"); a.Gate != "probe-mismatch:xtool" {
		t.Fatalf("%q", a.Gate)
	}
}
