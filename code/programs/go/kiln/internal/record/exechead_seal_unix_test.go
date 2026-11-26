//go:build unix

package record

// The exec-head choice is sealed at prepare (KILN04 D48, step 6 item 1).
//
//	prepare --exec-head   writes exec_head into the sealed binding and
//	                      --exec-head into pre/exec-wrapper.json
//	kiln record step      takes the choice from the seal; a flag of its own that
//	                      disagrees is refused before anything runs
//	collect               every STARTED step's sidecar has an exec-head nonce
//	                      exactly when the seal says on; a disagreement in either
//	                      direction voids the run the way a serial violation does
//
// No RecorderVersion changes: the accept set is the same, only a run that mixes
// supervised and unsupervised steps is refused at collect.

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const sealedSteps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
	`{"package":"code/packages/x/a","steps":["echo a"]},` +
	`{"package":"code/packages/x/b","steps":["echo b1","echo b2"]},` +
	`{"package":"code/packages/x/z","steps":["echo z"]}]}`

func wrapperOf(t *testing.T, dir string) []string {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(dir, "pre", "exec-wrapper.json"))
	if err != nil {
		t.Fatal(err)
	}
	var w []string
	if err := json.Unmarshal(data, &w); err != nil {
		t.Fatal(err)
	}
	return w
}

func TestPrepareSealsTheExecHeadChoice(t *testing.T) {
	for _, on := range []bool{false, true} {
		p := newPrep(t)
		p.execHead = on
		f := newStepFixOn(t, p, nil, planSteps, nil, nil, false, nil)
		if f.pre.Binding.ExecHead != on {
			t.Errorf("on=%v: the binding says exec_head=%v", on, f.pre.Binding.ExecHead)
		}
		raw, err := os.ReadFile(filepath.Join(p.dir, "pre", "binding.json"))
		if err != nil {
			t.Fatal(err)
		}
		// Absent when off: a default run's binding keeps its bytes (and its id).
		if got := bytes.Contains(raw, []byte(`"exec_head":true`)); got != on {
			t.Errorf("on=%v: binding.json mentions exec_head=true: %v (%s)", on, got, raw)
		}
		if bytes.Contains(raw, []byte(`"exec_head":false`)) {
			t.Errorf("an off seal must be absent, not false: %s", raw)
		}
		w := wrapperOf(t, p.dir)
		flag := ""
		for i, a := range w {
			if a == "--exec-head" {
				flag = a
				if i+1 >= len(w) || w[len(w)-1] != "--" {
					t.Errorf("the wrapper must end with the -- separator after the flag: %q", w)
				}
			}
		}
		if (flag != "") != on {
			t.Errorf("on=%v: the wrapper is %q", on, w)
		}
		if w[len(w)-1] != "--" || w[1] != "record" || w[2] != "step" {
			t.Errorf("the wrapper is not `kiln record step ... --`: %q", w)
		}
	}
}

func TestPrepareSealTravelsInTheBindingID(t *testing.T) {
	// The seal is part of the sealed bytes: the two choices cannot share an id.
	ids := map[bool]string{}
	for _, on := range []bool{false, true} {
		p := newPrep(t)
		p.execHead = on
		f := newStepFixOn(t, p, nil, planSteps, nil, nil, false, nil)
		ids[on] = f.pre.BindingID
	}
	if ids[false] == ids[true] {
		t.Fatal("an on seal and an off seal produced the same binding id")
	}
}

func TestStepRefusesAFlagThatDisagreesWithTheSeal(t *testing.T) {
	yes, no := true, false
	for _, c := range []struct {
		name   string
		sealed bool
		flag   *bool
	}{
		{"on flag against off seal", false, &yes},
		{"off flag against on seal", true, &no},
	} {
		t.Run(c.name, func(t *testing.T) {
			p := newPrep(t)
			p.execHead = c.sealed
			f := newStepFixOn(t, p, nil, planSteps, nil, nil, false, nil)
			o := f.opts(actA, 0, 1, "exit 0")
			o.ExecHeadFlag = c.flag
			f.refused(o, "disagrees with the sealed binding")
			// Nothing was claimed: no sidecar, so the step can still run, with the
			// seal's own choice, and the shard is not poisoned by the refusal.
			if _, err := os.Lstat(StepFile(f.p.dir, actA, 0, "json")); err == nil {
				t.Fatal("a refused step left a sidecar")
			}
		})
	}
}

func TestStepWithAnOffSealRunsWithoutASupervisor(t *testing.T) {
	f := newStepFix(t) // sealed off
	no := false
	o := f.opts(actA, 0, 1, "exit 0")
	o.ExecHeadFlag = &no // agrees with the seal
	if _, err := RunStep(o); err != nil {
		t.Fatalf("a flag that agrees with the seal: %v", err)
	}
	sc := f.sidecar(actA, 0)
	if sc.ExecHeadNonce != "" || sc.ExecHeadNotified != 0 || sc.ExecHeadKills != 0 {
		t.Fatalf("an off seal left exec-head fields: %+v", sc)
	}
	if _, err := os.Lstat(StepFile(f.p.dir, actA, 0, "exechead")); err == nil {
		t.Fatal("an off seal left an exec-head file")
	}
}

func TestStepSidecarExecHeadCountsAreValidated(t *testing.T) {
	withHead := func() StepSidecar {
		s := execHeadSidecar()
		s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = ehNonce, strings.Repeat("d", 64), 117
		return s
	}
	s := withHead()
	s.ExecHeadNotified, s.ExecHeadKills = 7, 1
	if _, err := EncodeStepSidecar(s); err != nil {
		t.Fatalf("counts beside a nonce: %v", err)
	}
	for name, mut := range map[string]func(*StepSidecar){
		"notified without a nonce": func(s *StepSidecar) {
			*s = execHeadSidecar()
			s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = "", "", 0
			s.ExecHeadNotified = 1
		},
		"kills without a nonce": func(s *StepSidecar) {
			*s = execHeadSidecar()
			s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = "", "", 0
			s.ExecHeadKills = 1
		},
		"a negative count": func(s *StepSidecar) { s.ExecHeadNotified = -1 },
	} {
		s := withHead()
		mut(&s)
		if _, err := EncodeStepSidecar(s); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
}

// ---------------------------------------------------------------------------
// collect

// newSealedCfix is a fresh one-shard world prepared with --exec-head (never a
// borrowed one: the borrowed worlds are all unsealed).
func newSealedCfix(t *testing.T, sealed bool) *cfix {
	t.Helper()
	requireGit(t)
	p := newPrep(t)
	p.execHead = sealed
	p.steps = sealedSteps
	p.commit()
	return finishCfix(t, p, "", singlePlan)
}

// stepWithHead writes step j of action; head says the sidecar carries an exec-head
// nonce and a bound exec-head file (as a supervised step's does).
func (f *cfix) stepWithHead(action string, j, seq int, head bool) {
	f.t.Helper()
	if !head {
		f.step(action, j, seq, nil)
		return
	}
	data := ehBytes(f.t, ehHeader(ehNonce, action, j))
	if err := os.MkdirAll(f.dirOf(action), 0o700); err != nil {
		f.t.Fatal(err)
	}
	if err := os.WriteFile(StepFile(f.p.dir, action, j, "exechead"), data, 0o600); err != nil {
		f.t.Fatal(err)
	}
	sum := sha256.Sum256(data)
	f.step(action, j, seq, func(sc *StepSidecar) {
		sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = ehNonce, hex.EncodeToString(sum[:]), int64(len(data))
		sc.ExecHeadNotified = 3
	})
}

// run writes a complete run of the three actions; head(action, j) says whether
// that step is supervised.
func (f *cfix) runWithHeads(head func(action string, j int) bool) {
	f.t.Helper()
	seq := 0
	for _, a := range []struct {
		name  string
		steps int
	}{{ca, 1}, {cb, 2}, {cz, 1}} {
		for j := 0; j < a.steps; j++ {
			f.stepWithHead(a.name, j, seq, head(a.name, j))
			seq++
		}
		f.verdict(a.name, a.steps, nil)
	}
	f.setSeq("4\n")
	f.setResults(
		ResultsPackage{Package: ca, Status: "built", StepsRun: 1, StepsTotal: 1},
		ResultsPackage{Package: cb, Status: "built", StepsRun: 2, StepsTotal: 2},
		ResultsPackage{Package: cz, Status: "built", StepsRun: 1, StepsTotal: 1},
	)
}

func TestCollectAcceptsAShardThatAgreesWithItsSeal(t *testing.T) {
	for _, on := range []bool{true, false} {
		f := newSealedCfix(t, on)
		f.runWithHeads(func(string, int) bool { return on })
		_, sh := f.collect()
		for _, a := range []string{ca, cb, cz} {
			requireKnown(t, sh, a)
		}
	}
}

func TestCollectVoidsAShardWhoseStepsDisagreeWithTheSeal(t *testing.T) {
	for _, c := range []struct {
		name   string
		sealed bool
		head   func(action string, j int) bool
	}{
		{"on seal and one step bare", true, func(a string, j int) bool { return !(a == cb && j == 1) }},
		{"on seal and no step supervised", true, func(string, int) bool { return false }},
		{"off seal and one step supervised", false, func(a string, j int) bool { return a == cz }},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := newSealedCfix(t, c.sealed)
			f.runWithHeads(c.head)
			_, sh := f.collect()
			// The run is void as a whole: EVERY action's write set is unknown, the
			// ones that agree with the seal included.
			for _, a := range []string{ca, cb, cz} {
				requireUnknown(t, sh, a, "exechead-seal")
			}
		})
	}
}

func TestCollectIgnoresAStepThatNeverStartedWhenCheckingTheSeal(t *testing.T) {
	f := newSealedCfix(t, true)
	// a's only step never started (strace could not be run): it has no supervisor,
	// and the seal does not make that a mismatch of its own.
	f.runWithHeads(func(a string, j int) bool { return a != ca })
	f.step(ca, 0, 0, func(sc *StepSidecar) {
		sc.Started, sc.StepNetNS, sc.Clean = false, false, false
		sc.Unclean = []string{UncleanNotStarted}
		sc.StraceExit, sc.RootStatus = -1, -1
		sc.StraceSignal = 0
		sc.Exit125 = false
	})
	_, sh := f.collect()
	for _, a := range []string{ca, cb, cz} {
		if r := recOf(t, sh, a); strings.Contains(r.Unknown, "exechead-seal") {
			t.Fatalf("%s: an unstarted step made a seal mismatch: %q", a, r.Unknown)
		}
	}
}

func TestExecHeadSealIsAKnownUnknownCode(t *testing.T) {
	if got := unknownReasonCode(reasonExecHeadMismatch); got != UnknownExecHeadSeal {
		t.Fatalf("the census files the mismatch under %q", got)
	}
}
