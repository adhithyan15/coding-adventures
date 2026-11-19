//go:build unix

package record

// The explain section on real record directories: the path-free reasons of
// record.json, parse reasons, record.failed categories and sidecar facts, with
// record.failed winning over a record.json and a foreign record refused.
// Unix only (the step fixture).

import (
	"os"
	"strings"
	"testing"
)

func putRecord(t *testing.T, f *stepFix, action string, mutate func(*ClassifiedRecord)) {
	t.Helper()
	rec := f.readRecord(action)
	mutate(&rec)
	data, err := EncodeClassified(rec)
	if err != nil {
		t.Fatal(err)
	}
	os.Remove(RecordFile(f.p.dir, action))
	writeBytes(t, RecordFile(f.p.dir, action), data, 0o644)
}

func TestCensusExplainsNonRecordableActions(t *testing.T) {
	f := censusDirFix(t)
	putRecord(t, f, actZ, func(r *ClassifiedRecord) {
		r.Record.Reasons = []string{"failed", "host-read:/etc/passwd", "host-read:/etc/shadow", "trace-incomplete:unstable-path:/a/b", "evil\n::error::x"}
		r.Record.WritesComplete = false
		r.WritesComplete = false
	})
	putRecord(t, f, actA, func(r *ClassifiedRecord) {
		r.ParseOK, r.Record, r.WritesComplete = false, nil, false
		r.ParseReasons = []string{"step 0: trace-incomplete:unparsable-line", "step 0: unclean:truncated", "step 1: trace-incomplete:unparsable-line"}
	})
	_, s, _, mdPath := runCensus(t, f)
	ex := s.Explain
	if ex.RecordReasonKinds["failed"] != 1 || ex.RecordReasonKinds["host-read"] != 1 || ex.RecordReasonKinds["trace-incomplete:unstable-path"] != 1 || ex.RecordReasonKinds["other"] != 1 || len(ex.RecordReasonKinds) != 4 {
		t.Errorf("reason kinds: %v", ex.RecordReasonKinds)
	}
	if ex.RecordParseReasons["unparsable-line"] != 1 || ex.RecordParseReasons["unclean:truncated"] != 1 || len(ex.RecordParseReasons) != 2 {
		t.Errorf("parse reasons: %v", ex.RecordParseReasons)
	}
	if ex.Sidecar.Steps != 2 || ex.Sidecar.NotClean != 0 || ex.Sidecar.Exit125 != 0 || ex.Sidecar.RootNonZero != 0 {
		t.Errorf("sidecar: %+v", ex.Sidecar)
	}
	md, _ := os.ReadFile(mdPath)
	for _, want := range []string{"## Why actions are not recordable", "`unparsable-line` 1;"} {
		if !strings.Contains(string(md), want) {
			t.Errorf("the Markdown lacks %q", want)
		}
	}
	if strings.Contains(string(md), "passwd") || strings.Contains(string(md), "::error::x") {
		t.Error("a reason's text reached the report")
	}
}

// record.failed wins over record.json (as in collect); each category is fixed.
func TestCensusRecordFailedWinsAndIsCategorised(t *testing.T) {
	f := censusDirFix(t)
	writeBytes(t, RecordFailedFile(f.p.dir, actA), []byte("timeout: the classifier ran longer than 10m0s and was killed"), 0o644)
	writeBytes(t, RecordFailedFile(f.p.dir, actZ), []byte("::error::pwn /home/x/secret"), 0o644)
	_, s, _, _ := runCensus(t, f)
	a := s.Actions
	if a.RecordFailed != 2 || a.WithRecord != 0 || a.NoVerdict != 0 || s.Problems["record_and_failed_both"] != 2 {
		t.Errorf("actions %+v, problems %v", a, s.Problems)
	}
	if s.Explain.RecordFailed["timeout"] != 1 || s.Explain.RecordFailed["other"] != 1 || len(s.Explain.RecordFailed) != 2 {
		t.Errorf("record.failed: %v", s.Explain.RecordFailed)
	}
	if l := s.Languages[0]; l.Recordable != 0 {
		t.Errorf("a failed action is recordable: %+v", l)
	}
}

// A record.json of another binding or sealed directory is counted and ignored.
func TestCensusRefusesAForeignRecord(t *testing.T) {
	f := censusDirFix(t)
	putRecord(t, f, actA, func(r *ClassifiedRecord) { r.BindingID = strings.Repeat("0", 64) })
	putRecord(t, f, actZ, func(r *ClassifiedRecord) { r.SealedSHA256 = strings.Repeat("1", 64) })
	_, s, _, _ := runCensus(t, f)
	if s.Problems["record_binding_mismatch"] != 2 || s.Actions.WithRecord != 0 || s.Actions.NoVerdict != 2 {
		t.Errorf("problems %v actions %+v", s.Problems, s.Actions)
	}
}
