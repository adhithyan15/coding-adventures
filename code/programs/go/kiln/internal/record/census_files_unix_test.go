//go:build unix

package record

// The per-file problems of `kiln record census` that are about traces, sidecars
// and records rather than about census.json (census_report_unix_test.go): each is
// counted under its code, the action is still counted, and nothing panics.
// Unix only: symlinks and POSIX paths.

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestCensusTraceAndSidecarProblems(t *testing.T) {
	for _, c := range []struct {
		name string
		code string
		put  func(t *testing.T, f *stepFix)
		// traces is how many traces are still parsed.
		traces int
	}{
		{"a symlinked trace", "trace_unreadable", func(t *testing.T, f *stepFix) {
			p := StepFile(f.p.dir, actA, 0, "trace")
			os.Remove(p)
			if err := os.Symlink(StepFile(f.p.dir, actZ, 0, "trace"), p); err != nil {
				t.Fatal(err)
			}
		}, 1},
		{"a trace that grew after the sidecar", "trace_size_differs_from_sidecar", func(t *testing.T, f *stepFix) {
			p := StepFile(f.p.dir, actA, 0, "trace")
			data, _ := os.ReadFile(p)
			os.Remove(p)
			writeBytes(t, p, append(data, []byte("10  getpid() = 10\n")...), 0o644)
		}, 2},
		{"a missing trace", "trace_missing", func(t *testing.T, f *stepFix) { os.Remove(StepFile(f.p.dir, actA, 0, "trace")) }, 1},
		{"a trace that is a directory", "trace_not_regular", func(t *testing.T, f *stepFix) {
			p := StepFile(f.p.dir, actA, 0, "trace")
			os.Remove(p)
			os.Mkdir(p, 0o755)
		}, 1},
		{"a garbage sidecar", "sidecar_invalid", func(t *testing.T, f *stepFix) {
			p := StepFile(f.p.dir, actA, 0, "json")
			os.Remove(p)
			writeBytes(t, p, []byte(`{"schema":"nope"}`), 0o644)
		}, 1},
		{"a sidecar of another action", "sidecar_action_mismatch", func(t *testing.T, f *stepFix) {
			data, _ := os.ReadFile(StepFile(f.p.dir, actZ, 0, "json"))
			p := StepFile(f.p.dir, actA, 0, "json")
			os.Remove(p)
			writeBytes(t, p, data, 0o644)
		}, 1},
		{"an oversized sidecar", "sidecar_unreadable", func(t *testing.T, f *stepFix) {
			p := StepFile(f.p.dir, actA, 0, "json")
			os.Remove(p)
			writeBytes(t, p, bytes.Repeat([]byte{' '}, maxSidecarBytes+1), 0o644)
		}, 1},
		{"a garbage record.json", "record_invalid", func(t *testing.T, f *stepFix) {
			p := RecordFile(f.p.dir, actA)
			os.Remove(p)
			writeBytes(t, p, []byte(`{"schema":"x"}`), 0o644)
		}, 2},
		{"a record.json of another action", "record_action_mismatch", func(t *testing.T, f *stepFix) {
			data, _ := os.ReadFile(RecordFile(f.p.dir, actZ))
			p := RecordFile(f.p.dir, actA)
			os.Remove(p)
			writeBytes(t, p, data, 0o644)
		}, 2},
		{"a symlinked record.json", "record_unreadable", func(t *testing.T, f *stepFix) {
			p := RecordFile(f.p.dir, actA)
			os.Remove(p)
			os.Symlink(RecordFile(f.p.dir, actZ), p)
		}, 2},
	} {
		t.Run(c.name, func(t *testing.T) {
			f := censusDirFix(t)
			c.put(t, f)
			res, s, _, _ := runCensus(t, f)
			if res.Exit != 0 || s.Problems[c.code] != 1 || s.Actions.Total != 2 || s.CrossCheck.Traces != c.traces {
				t.Errorf("exit %d, problems %v, actions %+v, traces %d (want %d)", res.Exit, s.Problems, s.Actions, s.CrossCheck.Traces, c.traces)
			}
		})
	}
}

// An action that is not in the sealed plan is counted, and its trace is not
// parsed with another action's parameters.
func TestCensusActionNotInPlan(t *testing.T) {
	f := censusDirFix(t)
	// A's files, copied under another action's name and directory.
	other := "code/packages/x/not-planned"
	src, dst := StepDir(f.p.dir, actA), StepDir(f.p.dir, other)
	if err := os.MkdirAll(dst, 0o755); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(src, "0.json"))
	sc, err := DecodeStepSidecar(data)
	if err != nil {
		t.Fatal(err)
	}
	sc.Action = other
	enc, err := EncodeStepSidecar(sc)
	if err != nil {
		t.Fatal(err)
	}
	writeBytes(t, filepath.Join(dst, "0.json"), enc, 0o644)
	tr, _ := os.ReadFile(filepath.Join(src, "0.trace"))
	writeBytes(t, filepath.Join(dst, "0.trace"), tr, 0o644)
	_, s, _, _ := runCensus(t, f)
	if s.Problems["action_not_in_plan"] != 1 || s.Actions.Total != 3 || s.CrossCheck.Traces != 2 || s.TraceSizes.All.N != 3 {
		t.Errorf("problems %v actions %+v cross %+v", s.Problems, s.Actions, s.CrossCheck)
	}
}
