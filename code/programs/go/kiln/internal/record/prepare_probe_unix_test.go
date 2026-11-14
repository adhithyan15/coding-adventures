//go:build unix

package record

// `prepare` and the structure probe (decision P14): where it runs, what it is
// given, and that nothing is sealed when it fails. The probe itself is a fake
// here (Options.Structure); the real one is exercised by the Linux end-to-end
// test TestSandboxRecordProbe.

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPrepareRunsTheProbeOnTheSealedConfiguration(t *testing.T) {
	p := newPrep(t)
	p.commit()
	res := p.mustPrepare()
	if len(p.probeIn) != 1 {
		t.Fatalf("the probe ran %d times", len(p.probeIn))
	}
	in := p.probeIn[0]
	first := res.Binding.SandboxConfigSHA256
	if in.ConfigSHA256 != first || in.ConfigPath != filepath.Join(res.PreDir, "sandbox.json") || in.Dir != p.dir {
		t.Fatalf("input %+v", in)
	}
	if in.Action != "code/packages/x/a" || in.KilnExe != p.kiln || in.Strace != p.strace {
		t.Fatalf("input %+v", in)
	}
	want := []string{"KILN_ACTION=code/packages/x/a", "KILN_STEP=0", "KILN_STEP_COUNT=1"}
	for i, w := range want {
		if in.Env[i] != w {
			t.Errorf("env[%d] = %q, want %q", i, in.Env[i], w)
		}
	}
	if !strings.HasPrefix(in.Env[3], "PATH=") {
		t.Errorf("env %v", in.Env)
	}
	// What was sealed is what the probe reported, and it is listed in SEALED.
	rep, err := DecodeProbes(readPre(t, res, ProbesFile))
	if err != nil {
		t.Fatal(err)
	}
	if rep.Action != in.Action || len(rep.Runs) != 3 {
		t.Fatalf("%+v", rep)
	}
	if !strings.Contains(string(readPre(t, res, "SEALED")), "  probes.json\n") {
		t.Fatal("probes.json is not sealed")
	}
	pre, err := VerifyPre(p.dir)
	if err != nil {
		t.Fatal(err)
	}
	if pre.Probes.Action != in.Action {
		t.Fatalf("%+v", pre.Probes)
	}
}

func TestPrepareRefusesAFailedOrWrongProbe(t *testing.T) {
	cases := map[string]struct {
		fn   func(ProbeInput) (ProbeReport, error)
		want string
	}{
		"the probe fails": {func(ProbeInput) (ProbeReport, error) { return ProbeReport{}, errors.New("a socket is reachable") }, "a socket is reachable"},
		"a probe is missing": {func(in ProbeInput) (ProbeReport, error) {
			r := fakeProbeReport(in)
			r.Runs = r.Runs[:2]
			return r, nil
		}, "runs, want"},
		"another action": {func(in ProbeInput) (ProbeReport, error) {
			r := fakeProbeReport(in)
			r.Action = "code/packages/x/z"
			return r, nil
		}, "not for the first action"},
		"a non-zero exit": {func(in ProbeInput) (ProbeReport, error) {
			r := fakeProbeReport(in)
			r.Runs[1].Exit = 2
			return r, nil
		}, "exited 2"},
		"an empty report": {func(ProbeInput) (ProbeReport, error) { return ProbeReport{}, nil }, "schema"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			p := newPrep(t)
			p.commit()
			p.probeFn = c.fn
			_, err := p.prepare()
			if err == nil || !strings.Contains(err.Error(), "structure probe") || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("%v", err)
			}
			if _, statErr := os.Lstat(filepath.Join(p.dir, "pre", "SEALED")); statErr == nil {
				t.Fatal("a failed probe left a sealed record")
			}
			if _, statErr := os.Lstat(filepath.Join(p.dir, "pre", ProbesFile)); statErr == nil {
				t.Fatal("a failed probe left a probes.json")
			}
		})
	}
}

func TestPrepareDoesNotProbeBeforeTheHostChecksPass(t *testing.T) {
	p := newPrep(t)
	p.commit()
	p.env = append(p.env, "LD_PRELOAD=/x") // refused by LoaderEnv long before
	if _, err := p.prepare(); err == nil {
		t.Skip("the loader variable was accepted; the ordering assertion below would be vacuous")
	}
	if len(p.probeIn) != 0 {
		t.Fatal("the probe ran although an earlier check failed")
	}
}

func TestVerifyPreChecksProbesAgainstTheConfiguration(t *testing.T) {
	for name, edit := range map[string]func(r *ProbeReport){
		"a dropped probe":    func(r *ProbeReport) { r.Runs = r.Runs[:1] },
		"an unknown action":  func(r *ProbeReport) { r.Action = "code/packages/x/nope" },
		"a changed argv":     func(r *ProbeReport) { r.Runs[0].Argv = []string{"true"} },
		"another trace size": func(r *ProbeReport) { r.Runs[0].TraceBytes = -1 },
	} {
		t.Run(name, func(t *testing.T) {
			f := newStepFix(t)
			f.p.writable()
			data, err := os.ReadFile(filepath.Join(f.p.dir, "pre", ProbesFile))
			if err != nil {
				t.Fatal(err)
			}
			rep, err := DecodeProbes(data)
			if err != nil {
				t.Fatal(err)
			}
			edit(&rep)
			bad, err := EncodeProbes(rep)
			if err != nil {
				t.Fatal(err)
			}
			f.reseal(map[string][]byte{ProbesFile: bad}, false)
			if _, err := VerifyPre(f.p.dir); err == nil || !strings.Contains(err.Error(), "probes") {
				t.Fatalf("%v", err)
			}
		})
	}
	t.Run("not canonical", func(t *testing.T) {
		f := newStepFix(t)
		f.reseal(map[string][]byte{ProbesFile: []byte("{}")}, false)
		if _, err := VerifyPre(f.p.dir); err == nil || !strings.Contains(err.Error(), "probes") {
			t.Fatalf("%v", err)
		}
	})
}
